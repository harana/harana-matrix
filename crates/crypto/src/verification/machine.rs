// Copyright 2020 The Matrix.org Foundation C.I.C.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{collections::HashMap, sync::Arc};

use sdk_common::locks::RwLock as StdRwLock;
use ruma::{
    DeviceId, EventId, MilliSecondsSinceUnixEpoch, OwnedDeviceId, OwnedUserId, RoomId,
    SecondsSinceUnixEpoch, TransactionId, UInt, UserId,
    events::{
        AnyToDeviceEvent, AnyToDeviceEventContent, ToDeviceEvent,
        key::verification::{
            VerificationMethod,
            ready::{KeyVerificationReadyEventContent, ToDeviceKeyVerificationReadyEventContent},
            request::ToDeviceKeyVerificationRequestEventContent,
        },
        room::message::KeyVerificationRequestEventContent,
    },
    serde::Raw,
    uint,
};
use tokio::sync::Mutex;
use tracing::{Span, debug, info, instrument, trace, warn};

use super::{
    FlowId, Verification, VerificationResult, VerificationStore,
    cache::{RequestInfo, VerificationCache},
    event_enums::{
        AnyEvent, AnyVerificationContent, OutgoingContent, ReadyContent, RequestContent,
    },
    requests::VerificationRequest,
    sas::Sas,
};
use crate::{
    DeviceData, OtherUserIdentityData,
    olm::{PrivateCrossSigningIdentity, StaticAccountData},
    store::{CryptoStoreError, CryptoStoreWrapper},
    types::requests::{
        OutgoingRequest, OutgoingVerificationRequest, RoomMessageRequest, ToDeviceRequest,
    },
};

/// A verification event received for a device we don't know about yet.
///
/// See [`VerificationMachine::pending_requests`].
#[derive(Clone, Debug)]
struct PendingRequest {
    sender: OwnedUserId,
    flow_id: FlowId,
    content: OwnedPendingContent,
    timestamp: MilliSecondsSinceUnixEpoch,
}

impl PendingRequest {
    fn sending_device(&self) -> &DeviceId {
        self.content.sending_device()
    }
}

/// An owned version of the verification contents that need the sending device
/// to be known, so that one can be kept around until the device list catches
/// up.
///
/// [`RequestContent`]: super::event_enums::RequestContent
#[derive(Clone, Debug)]
enum OwnedPendingContent {
    /// An `m.key.verification.request`, i.e. somebody wants to verify with us.
    RequestToDevice(ToDeviceKeyVerificationRequestEventContent),
    /// An in-room `m.key.verification.request`.
    RequestRoom(KeyVerificationRequestEventContent),
    /// An `m.key.verification.ready`, i.e. the other side accepted a request we
    /// sent.
    ReadyToDevice(ToDeviceKeyVerificationReadyEventContent),
    /// An in-room `m.key.verification.ready`.
    ReadyRoom(KeyVerificationReadyEventContent),
}

impl OwnedPendingContent {
    fn sending_device(&self) -> &DeviceId {
        match self {
            Self::RequestToDevice(content) => &content.from_device,
            Self::RequestRoom(content) => &content.from_device,
            Self::ReadyToDevice(content) => &content.from_device,
            Self::ReadyRoom(content) => &content.from_device,
        }
    }

    fn as_request(&self) -> Option<RequestContent<'_>> {
        match self {
            Self::RequestToDevice(content) => Some(RequestContent::ToDevice(content)),
            Self::RequestRoom(content) => Some(RequestContent::Room(content)),
            Self::ReadyToDevice(_) | Self::ReadyRoom(_) => None,
        }
    }

    fn as_ready(&self) -> Option<ReadyContent<'_>> {
        match self {
            Self::ReadyToDevice(content) => Some(ReadyContent::ToDevice(content)),
            Self::ReadyRoom(content) => Some(ReadyContent::Room(content)),
            Self::RequestToDevice(_) | Self::RequestRoom(_) => None,
        }
    }
}

impl From<&RequestContent<'_>> for OwnedPendingContent {
    fn from(value: &RequestContent<'_>) -> Self {
        match value {
            RequestContent::ToDevice(content) => Self::RequestToDevice((*content).clone()),
            RequestContent::Room(content) => Self::RequestRoom((*content).clone()),
        }
    }
}

impl From<&ReadyContent<'_>> for OwnedPendingContent {
    fn from(value: &ReadyContent<'_>) -> Self {
        match value {
            ReadyContent::ToDevice(content) => Self::ReadyToDevice((*content).clone()),
            ReadyContent::Room(content) => Self::ReadyRoom((*content).clone()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct VerificationMachine {
    pub(crate) store: VerificationStore,
    verifications: VerificationCache,
    requests: Arc<StdRwLock<HashMap<OwnedUserId, HashMap<String, VerificationRequest>>>>,

    /// Verification requests received from a device we didn't know about yet.
    ///
    /// A verification request is a to-device event, and it can arrive in the
    /// same sync response as the device list update that tells us about the
    /// sending device — or even before it. The device only lands in the store
    /// once the `/keys/query` that update triggers has been answered, which is
    /// after the to-device event has been processed. This is common after the
    /// app has been backgrounded.
    ///
    /// Rather than dropping such a request, it waits here until the device
    /// list catches up; see
    /// [`VerificationMachine::retry_pending_requests()`].
    pending_requests: Arc<StdRwLock<Vec<PendingRequest>>>,
}

impl VerificationMachine {
    pub(crate) fn new(
        account: StaticAccountData,
        identity: Arc<Mutex<PrivateCrossSigningIdentity>>,
        store: Arc<CryptoStoreWrapper>,
    ) -> Self {
        Self {
            store: VerificationStore { account, private_identity: identity, inner: store },
            verifications: VerificationCache::new(),
            requests: Default::default(),
            pending_requests: Default::default(),
        }
    }

    pub(crate) fn own_user_id(&self) -> &UserId {
        &self.store.account.user_id
    }

    pub(crate) fn own_device_id(&self) -> &DeviceId {
        &self.store.account.device_id
    }

    pub(crate) fn request_to_device_verification(
        &self,
        user_id: &UserId,
        recipient_devices: Vec<OwnedDeviceId>,
        methods: Option<Vec<VerificationMethod>>,
    ) -> (VerificationRequest, OutgoingVerificationRequest) {
        let flow_id = FlowId::from(TransactionId::new());

        let verification = VerificationRequest::new(
            self.verifications.clone(),
            self.store.clone(),
            flow_id,
            user_id,
            recipient_devices,
            methods,
        );

        self.insert_request(verification.clone());

        let request = verification.request_to_device();

        (verification, request.into())
    }

    pub fn request_verification(
        &self,
        identity: &OtherUserIdentityData,
        room_id: &RoomId,
        request_event_id: &EventId,
        methods: Option<Vec<VerificationMethod>>,
    ) -> VerificationRequest {
        let flow_id = FlowId::InRoom(room_id.to_owned(), request_event_id.to_owned());

        let request = VerificationRequest::new(
            self.verifications.clone(),
            self.store.clone(),
            flow_id,
            identity.user_id(),
            vec![],
            methods,
        );

        self.insert_request(request.clone());

        request
    }

    pub async fn start_sas(
        &self,
        device: DeviceData,
    ) -> Result<(Sas, OutgoingVerificationRequest), CryptoStoreError> {
        let identities = self.store.get_identities(device.clone()).await?;
        let (sas, content) = Sas::start(identities, TransactionId::new(), true, None, None);

        let request = match content {
            OutgoingContent::Room(r, c) => {
                RoomMessageRequest { room_id: r, txn_id: TransactionId::new(), content: c }.into()
            }
            OutgoingContent::ToDevice(c) => {
                let request = ToDeviceRequest::with_id(
                    device.user_id(),
                    device.device_id().to_owned(),
                    &c,
                    TransactionId::new(),
                );

                self.verifications.insert_sas(sas.clone());

                request.into()
            }
        };

        Ok((sas, request))
    }

    pub fn get_request(
        &self,
        user_id: &UserId,
        flow_id: impl AsRef<str>,
    ) -> Option<VerificationRequest> {
        self.requests.read().get(user_id)?.get(flow_id.as_ref()).cloned()
    }

    pub fn get_requests(&self, user_id: &UserId) -> Vec<VerificationRequest> {
        self.requests.read().get(user_id).map(|v| v.values().cloned().collect()).unwrap_or_default()
    }

    /// Add a new `VerificationRequest` object to the cache.
    /// If there are any existing requests with this user (and different
    /// flow_id), both the existing and new request will be cancelled.
    fn insert_request(&self, request: VerificationRequest) {
        if let Some(r) = self.get_request(request.other_user(), request.flow_id().as_str()) {
            debug!(flow_id = r.flow_id().as_str(), "Ignoring known verification request",);
            return;
        }

        let mut requests = self.requests.write();
        let user_requests = requests.entry(request.other_user().to_owned()).or_default();

        // Cancel all the old verifications requests as well as the new one we
        // have for this user if someone tries to have two verifications going
        // on at once.
        for old_verification in user_requests.values_mut() {
            if !old_verification.is_cancelled() {
                warn!(
                    "Received a new verification request whilst another request \
                    with the same user is ongoing. Cancelling both requests."
                );

                if let Some(r) = old_verification.cancel() {
                    self.verifications.add_request(r.into())
                }

                if let Some(r) = request.cancel() {
                    self.verifications.add_request(r.into())
                }
            }
        }

        // We still want to add the new verification request, in case users
        // want to inspect the verification object a matching
        // `m.key.verification.request` produced.
        user_requests.insert(request.flow_id().as_str().to_owned(), request);
    }

    pub fn get_verification(&self, user_id: &UserId, flow_id: &str) -> Option<Verification> {
        self.verifications.get(user_id, flow_id)
    }

    pub fn get_sas(&self, user_id: &UserId, flow_id: &str) -> Option<Box<Sas>> {
        self.verifications.get_sas(user_id, flow_id)
    }

    fn is_timestamp_valid(timestamp: MilliSecondsSinceUnixEpoch) -> bool {
        // The event should be ignored if the event is older than 10 minutes
        let old_timestamp_threshold: UInt = uint!(600);
        // The event should be ignored if the event is 5 minutes or more into the
        // future.
        let timestamp_threshold: UInt = uint!(300);

        let timestamp = timestamp.as_secs();
        let now = SecondsSinceUnixEpoch::now().get();

        !(now.saturating_sub(timestamp) > old_timestamp_threshold
            || timestamp.saturating_sub(now) > timestamp_threshold)
    }

    fn queue_up_content(
        &self,
        recipient: &UserId,
        recipient_device: &DeviceId,
        content: OutgoingContent,
        request_id: Option<RequestInfo>,
    ) {
        self.verifications.queue_up_content(recipient, recipient_device, content, request_id)
    }

    pub fn mark_request_as_sent(&self, request_id: &TransactionId) {
        self.verifications.mark_request_as_sent(request_id);
    }

    pub fn outgoing_messages(&self) -> Vec<OutgoingRequest> {
        self.verifications.outgoing_requests()
    }

    pub fn garbage_collect(&self) -> Vec<Raw<AnyToDeviceEvent>> {
        let mut events = vec![];

        let mut requests: Vec<OutgoingVerificationRequest> = {
            let mut requests = self.requests.write();

            for user_verification in requests.values_mut() {
                user_verification.retain(|_, v| !(v.is_done() || v.is_cancelled()));
            }
            requests.retain(|_, v| !v.is_empty());

            requests.values().flatten().filter_map(|(_, v)| v.cancel_if_timed_out()).collect()
        };

        requests.extend(self.verifications.garbage_collect());

        for request in requests {
            if let Ok(OutgoingContent::ToDevice(to_device)) = request.clone().try_into()
                && let AnyToDeviceEventContent::KeyVerificationCancel(content) = *to_device
            {
                let event = ToDeviceEvent::new(self.own_user_id().to_owned(), content);

                events.push(
                    Raw::new(&event)
                        .expect("Failed to serialize m.key_verification.cancel event")
                        .cast(),
                );
            }

            self.verifications.add_verification_request(request)
        }

        events
    }

    async fn mark_sas_as_done(
        &self,
        sas: &Sas,
        out_content: Option<OutgoingContent>,
    ) -> Result<(), CryptoStoreError> {
        match sas.mark_as_done().await? {
            VerificationResult::Ok => {
                if let Some(c) = out_content {
                    self.queue_up_content(sas.other_user_id(), sas.other_device_id(), c, None);
                }
            }
            VerificationResult::Cancel(c) => {
                if let Some(r) = sas.cancel_with_code(c) {
                    self.verifications.add_request(r.into());
                }
            }
            VerificationResult::SignatureUpload(r) => {
                self.verifications.add_request(r.into());

                if let Some(c) = out_content {
                    self.queue_up_content(sas.other_user_id(), sas.other_device_id(), c, None);
                }
            }
        }

        Ok(())
    }

    /// The maximum number of verification requests kept while waiting for the
    /// device list to catch up.
    ///
    /// Verification requests expire after 10 minutes anyway (see
    /// [`Self::is_timestamp_valid`]), this is only there to bound the memory a
    /// misbehaving server can make us use.
    const MAX_PENDING_REQUESTS: usize = 20;

    fn remember_pending_request(&self, pending_request: PendingRequest) {
        let mut pending_requests = self.pending_requests.write();

        // Replace a request for the same flow, if there is one, so that a repeated
        // request doesn't pile up.
        pending_requests.retain(|request| {
            request.flow_id != pending_request.flow_id || request.sender != pending_request.sender
        });

        if pending_requests.len() >= Self::MAX_PENDING_REQUESTS {
            pending_requests.remove(0);
        }

        pending_requests.push(pending_request);
    }

    /// Retry the verification requests that arrived before we knew about the
    /// device that sent them.
    ///
    /// This is called once the device list has caught up, i.e. after a
    /// `/keys/query` response has been processed. Requests whose device is
    /// still unknown are kept for the next round, until they expire.
    pub(crate) async fn retry_pending_requests(&self) -> Result<(), CryptoStoreError> {
        let pending_requests = {
            let mut guard = self.pending_requests.write();

            if guard.is_empty() {
                return Ok(());
            }

            std::mem::take(&mut *guard)
        };

        let mut still_pending = Vec::new();

        for pending_request in pending_requests {
            if !Self::is_timestamp_valid(pending_request.timestamp) {
                debug!(
                    ?pending_request,
                    "Dropping a pending verification request, it is too old now"
                );
                continue;
            }

            let Some(device_data) = self
                .store
                .get_device(&pending_request.sender, pending_request.sending_device())
                .await?
            else {
                still_pending.push(pending_request.clone());
                continue;
            };

            info!(
                sender = ?pending_request.sender,
                from_device = pending_request.sending_device().as_str(),
                "The device list caught up with a verification event we had put aside",
            );

            if let Some(content) = pending_request.content.as_request() {
                let request = VerificationRequest::from_request(
                    self.verifications.clone(),
                    self.store.clone(),
                    &pending_request.sender,
                    pending_request.flow_id.clone(),
                    &content,
                    device_data,
                );

                self.insert_request(request);
            } else if let Some(content) = pending_request.content.as_ready() {
                let Some(request) =
                    self.get_request(&pending_request.sender, pending_request.flow_id.as_str())
                else {
                    // The request went away while we were waiting; nothing to accept.
                    continue;
                };

                if request.flow_id() == &pending_request.flow_id {
                    request.receive_ready(&pending_request.sender, &content, device_data);
                }
            }
        }

        self.pending_requests.write().extend(still_pending);

        Ok(())
    }

    #[instrument(skip_all, fields(flow_id))]
    pub async fn receive_any_event(
        &self,
        event: impl Into<AnyEvent<'_>>,
    ) -> Result<(), CryptoStoreError> {
        let event = event.into();

        let Ok(flow_id) = FlowId::try_from(&event) else {
            // This isn't a verification event, return early.
            return Ok(());
        };
        Span::current().record("flow_id", flow_id.as_str());

        let flow_id_mismatch = || {
            warn!(
                flow_id = flow_id.as_str(),
                "Received a verification event with a mismatched flow id, \
                 the verification object was created for a in-room \
                 verification but an event was received over to-device \
                 messaging or vice versa"
            );
        };

        let event_sent_from_us = |event: &AnyEvent<'_>, from_device: &DeviceId| {
            if event.sender() == self.store.account.user_id {
                from_device == self.store.account.device_id || event.is_room_event()
            } else {
                false
            }
        };

        let Some(content) = event.verification_content() else { return Ok(()) };
        match &content {
            AnyVerificationContent::Request(r) => {
                info!(
                    sender = ?event.sender(),
                    from_device = r.from_device().as_str(),
                    "Received a new verification request",
                );

                let Some(timestamp) = event.timestamp() else {
                    warn!(
                        from_device = r.from_device().as_str(),
                        "The key verification request didn't contain a valid timestamp"
                    );
                    return Ok(());
                };

                if !Self::is_timestamp_valid(timestamp) {
                    info!(
                        from_device = r.from_device().as_str(),
                        ?timestamp,
                        "The received verification request was too old or too far into the future",
                    );
                    return Ok(());
                }

                if event_sent_from_us(&event, r.from_device()) {
                    trace!(
                        from_device = r.from_device().as_str(),
                        "The received verification request was sent by us, ignoring it",
                    );
                    return Ok(());
                }

                let Some(device_data) =
                    self.store.get_device(event.sender(), r.from_device()).await?
                else {
                    // We don't know that device yet. The device list update announcing
                    // it may well be in this very sync response, in which case the
                    // `/keys/query` it triggers hasn't been answered yet. Keep the
                    // request around instead of dropping it, and retry once the device
                    // list has caught up.
                    debug!(
                        from_device = r.from_device().as_str(),
                        "Don't know the device the verification request came from yet; \
                         waiting for the device list to catch up"
                    );

                    self.remember_pending_request(PendingRequest {
                        sender: event.sender().to_owned(),
                        flow_id,
                        content: r.into(),
                        timestamp,
                    });

                    return Ok(());
                };

                let request = VerificationRequest::from_request(
                    self.verifications.clone(),
                    self.store.clone(),
                    event.sender(),
                    flow_id,
                    r,
                    device_data,
                );

                self.insert_request(request);
            }
            AnyVerificationContent::Cancel(c) => {
                if let Some(verification) = self.get_request(event.sender(), flow_id.as_str()) {
                    verification.receive_cancel(event.sender(), c);
                }

                if let Some(verification) = self.get_verification(event.sender(), flow_id.as_str())
                {
                    match verification {
                        Verification::SasV1(sas) => {
                            // This won't produce an outgoing content
                            let _ = sas.receive_any_event(event.sender(), &content);
                        }
                        #[cfg(feature = "qrcode")]
                        Verification::QrV1(qr) => qr.receive_cancel(event.sender(), c),
                    }
                }
            }
            AnyVerificationContent::Ready(c) => {
                let Some(request) = self.get_request(event.sender(), flow_id.as_str()) else {
                    return Ok(());
                };

                if request.flow_id() == &flow_id {
                    if let Some(device_data) =
                        self.store.get_device(event.sender(), c.from_device()).await?
                    {
                        request.receive_ready(event.sender(), c, device_data);
                    } else {
                        // We don't know the accepting device yet: the `/keys/query` that
                        // would tell us about it is still in flight. Dropping the ready
                        // here used to leave the request stuck in the `Created` state, so
                        // a client that went on to call `start_sas()` or
                        // `generate_qr_code()` got nothing back. Keep it and replay it
                        // once the device list has caught up.
                        debug!(
                            from_device = c.from_device().as_str(),
                            "Don't know the device that accepted the verification yet; \
                             waiting for the device list to catch up"
                        );

                        self.remember_pending_request(PendingRequest {
                            sender: event.sender().to_owned(),
                            flow_id,
                            content: c.into(),
                            timestamp: MilliSecondsSinceUnixEpoch::now(),
                        });
                    }
                } else {
                    flow_id_mismatch();
                }
            }
            AnyVerificationContent::Start(c) => {
                if let Some(request) = self.get_request(event.sender(), flow_id.as_str()) {
                    if request.flow_id() == &flow_id {
                        Box::pin(request.receive_start(event.sender(), c)).await?
                    } else {
                        flow_id_mismatch();
                    }
                } else if let FlowId::ToDevice(_) = flow_id {
                    // TODO remove this soon, this has been deprecated by
                    // MSC3122 https://github.com/matrix-org/matrix-doc/pull/3122
                    if let Some(device) =
                        self.store.get_device(event.sender(), c.from_device()).await?
                    {
                        let identities = self.store.get_identities(device).await?;

                        match Sas::from_start_event(flow_id, c, identities, None, false) {
                            Ok(sas) => {
                                self.verifications.insert_sas(sas);
                            }
                            Err(cancellation) => self.queue_up_content(
                                event.sender(),
                                c.from_device(),
                                cancellation,
                                None,
                            ),
                        }
                    }
                }
            }
            AnyVerificationContent::Accept(_) | AnyVerificationContent::Key(_) => {
                let Some(sas) = self.get_sas(event.sender(), flow_id.as_str()) else {
                    return Ok(());
                };

                if sas.flow_id() != &flow_id {
                    flow_id_mismatch();
                    return Ok(());
                }

                let Some((content, request_info)) = sas.receive_any_event(event.sender(), &content)
                else {
                    return Ok(());
                };

                self.queue_up_content(
                    sas.other_user_id(),
                    sas.other_device_id(),
                    content,
                    request_info,
                );
            }
            AnyVerificationContent::Mac(_) => {
                let Some(s) = self.get_sas(event.sender(), flow_id.as_str()) else { return Ok(()) };

                if s.flow_id() != &flow_id {
                    flow_id_mismatch();
                    return Ok(());
                }

                let content = s.receive_any_event(event.sender(), &content);

                if s.is_done() {
                    Box::pin(self.mark_sas_as_done(&s, content.map(|(c, _)| c))).await?;
                } else {
                    // Even if we are not done (yet), there might be content to
                    // send out, e.g. in the case where we are done with our
                    // side of the verification process, but the other side has
                    // not yet sent their "done".
                    let Some((content, request_id)) = content else { return Ok(()) };

                    self.queue_up_content(
                        s.other_user_id(),
                        s.other_device_id(),
                        content,
                        request_id,
                    );
                }
            }
            AnyVerificationContent::Done(c) => {
                if let Some(verification) = self.get_request(event.sender(), flow_id.as_str()) {
                    verification.receive_done(event.sender(), c);
                }

                #[allow(clippy::single_match)]
                match self.get_verification(event.sender(), flow_id.as_str()) {
                    Some(Verification::SasV1(sas)) => {
                        let content = sas.receive_any_event(event.sender(), &content);

                        if sas.is_done() {
                            Box::pin(self.mark_sas_as_done(&sas, content.map(|(c, _)| c))).await?;
                        }
                    }
                    #[cfg(feature = "qrcode")]
                    Some(Verification::QrV1(qr)) => {
                        let (cancellation, request) = Box::pin(qr.receive_done(c)).await?;

                        if let Some(c) = cancellation {
                            self.verifications.add_request(c.into())
                        }

                        if let Some(s) = request {
                            self.verifications.add_request(s.into())
                        }
                    }
                    None => {}
                }
            }
        }

        Ok(())
    }

    /**
     * Utility function to build the public identity (i.e., an
     * [`OwnUserIdentityData`]) corresponding to the private identity
     * stored in the `VerificationStore`.
     */
    #[cfg(test)]
    pub async fn get_own_user_identity_data(
        &self,
    ) -> Result<crate::OwnUserIdentityData, crate::SignatureError> {
        self.store.private_identity.lock().await.to_public_identity().await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sdk_test::async_test;
    use ruma::TransactionId;
    use tokio::sync::Mutex;

    use super::{Sas, VerificationMachine};
    use crate::{
        Account, VerificationRequest,
        olm::PrivateCrossSigningIdentity,
        store::{CryptoStoreWrapper, MemoryStore},
        verification::{
            FlowId, VerificationStore,
            cache::VerificationCache,
            event_enums::{AcceptContent, KeyContent, MacContent, OutgoingContent},
            tests::{alice_device_id, alice_id, setup_stores, wrap_any_to_device_content},
        },
    };

    async fn verification_machine() -> (VerificationMachine, VerificationStore) {
        let (_account, store, _bob, bob_store) = setup_stores().await;

        let machine = VerificationMachine {
            store,
            verifications: VerificationCache::new(),
            requests: Default::default(),
            pending_requests: Default::default(),
        };

        (machine, bob_store)
    }

    async fn setup_verification_machine() -> (VerificationMachine, Sas) {
        let (machine, bob_store) = verification_machine().await;

        let alice_device =
            bob_store.get_device(alice_id(), alice_device_id()).await.unwrap().unwrap();

        let identities = bob_store.get_identities(alice_device).await.unwrap();
        let (bob_sas, start_content) =
            Sas::start(identities, TransactionId::new(), true, None, None);

        machine
            .receive_any_event(&wrap_any_to_device_content(bob_sas.user_id(), start_content))
            .await
            .unwrap();

        (machine, bob_sas)
    }

    #[async_test]
    async fn test_create() {
        let alice = Account::with_device_id(alice_id(), alice_device_id());
        let identity = Arc::new(Mutex::new(PrivateCrossSigningIdentity::empty(alice_id())));
        let _ = VerificationMachine::new(
            alice.static_data,
            identity,
            Arc::new(CryptoStoreWrapper::new(alice_id(), alice_device_id(), MemoryStore::new())),
        );
    }

    #[async_test]
    async fn test_verification_request_from_an_unknown_device_is_retried() {
        use ruma::{
            MilliSecondsSinceUnixEpoch, device_id,
            events::{ToDeviceEvent, key::verification::VerificationMethod},
            user_id,
        };

        use crate::{
            DeviceData,
            store::types::{Changes, DeviceChanges},
            types::events::ToDeviceEvents,
        };

        let (machine, _bob_store) = verification_machine().await;

        // A device the machine doesn't know about yet: this is what the sending device
        // of a verification request looks like when the request arrives before, or in
        // the same sync response as, the device list update announcing it.
        let carol = Account::with_device_id(user_id!("@carol:example.org"), device_id!("CAROLDEV"));
        let carol_device = DeviceData::from_account(&carol);

        let flow_id = TransactionId::new();
        let content = ruma::events::key::verification::request::ToDeviceKeyVerificationRequestEventContent::new(
            carol.device_id().to_owned(),
            flow_id.clone(),
            vec![VerificationMethod::SasV1],
            MilliSecondsSinceUnixEpoch::now(),
        );
        let event = ToDeviceEvents::KeyVerificationRequest(ToDeviceEvent::new(
            carol.user_id().to_owned(),
            content,
        ));

        machine.receive_any_event(&event).await.unwrap();

        // The request isn't visible yet, we don't know the device it came from.
        assert!(machine.get_request(carol.user_id(), flow_id.as_str()).is_none());

        // The `/keys/query` triggered by the device list update is answered…
        machine
            .store
            .inner
            .save_changes(Changes {
                devices: DeviceChanges { new: vec![carol_device], ..Default::default() },
                ..Default::default()
            })
            .await
            .unwrap();

        machine.retry_pending_requests().await.unwrap();

        // … and the request we had put aside is picked up instead of being lost.
        assert!(machine.get_request(carol.user_id(), flow_id.as_str()).is_some());
    }

    #[async_test]
    async fn test_verification_ready_from_an_unknown_device_is_retried() {
        use ruma::{
            device_id,
            events::{
                ToDeviceEvent,
                key::verification::{
                    VerificationMethod, ready::ToDeviceKeyVerificationReadyEventContent,
                },
            },
            user_id,
        };

        use crate::{
            DeviceData,
            store::types::{Changes, DeviceChanges},
            types::events::ToDeviceEvents,
        };

        let (machine, _bob_store) = verification_machine().await;

        // Given a verification request we sent to another user,
        let carol_id = user_id!("@carol:example.org");
        let carol = Account::with_device_id(carol_id, device_id!("CAROLDEV"));
        let carol_device = DeviceData::from_account(&carol);

        let (request, _) = machine.request_to_device_verification(
            carol_id,
            vec![carol.device_id().to_owned()],
            Some(vec![VerificationMethod::SasV1]),
        );
        let flow_id = request.flow_id().as_str().to_owned();

        assert!(!request.is_ready());

        // When Carol accepts it from a device we don't know about yet,
        let content = ToDeviceKeyVerificationReadyEventContent::new(
            carol.device_id().to_owned(),
            vec![VerificationMethod::SasV1],
            flow_id.as_str().into(),
        );
        let event =
            ToDeviceEvents::KeyVerificationReady(ToDeviceEvent::new(carol_id.to_owned(), content));

        machine.receive_any_event(&event).await.unwrap();

        // Then the request is not ready yet, because we can't attribute the acceptance
        // to a device.
        assert!(!request.is_ready());

        // But once the `/keys/query` answer lands,
        machine
            .store
            .inner
            .save_changes(Changes {
                devices: DeviceChanges { new: vec![carol_device], ..Default::default() },
                ..Default::default()
            })
            .await
            .unwrap();

        machine.retry_pending_requests().await.unwrap();

        // … the acceptance we had put aside is applied instead of being lost.
        assert!(request.is_ready());
    }

    #[async_test]
    async fn test_full_flow() {
        let (alice_machine, bob) = setup_verification_machine().await;

        let alice = alice_machine.get_sas(bob.user_id(), bob.flow_id().as_str()).unwrap();

        let request = alice.accept().unwrap();

        let content = OutgoingContent::try_from(request).unwrap();
        let content = AcceptContent::try_from(&content).unwrap().into();

        let (content, request_info) = bob.receive_any_event(alice.user_id(), &content).unwrap();

        let event = wrap_any_to_device_content(bob.user_id(), content);

        assert!(alice_machine.verifications.outgoing_requests().is_empty());
        alice_machine.receive_any_event(&event).await.unwrap();
        assert!(!alice_machine.verifications.outgoing_requests().is_empty());

        let request = alice_machine.verifications.outgoing_requests().first().cloned().unwrap();
        let txn_id = request.request_id().to_owned();
        let content = OutgoingContent::try_from(request).unwrap();
        let content = KeyContent::try_from(&content).unwrap().into();

        alice_machine.mark_request_as_sent(&txn_id);

        assert!(bob.receive_any_event(alice.user_id(), &content).is_none());

        assert!(alice.emoji().is_some());
        // Bob can only show the emoji if it marks the request carrying the
        // m.key.verification.key event as sent.
        assert!(bob.emoji().is_none());
        bob.mark_request_as_sent(&request_info.unwrap().request_id);
        assert!(bob.emoji().is_some());
        assert_eq!(alice.emoji(), bob.emoji());

        let mut requests = alice.confirm().await.unwrap().0;
        assert!(requests.len() == 1);
        let request = requests.pop().unwrap();
        let content = OutgoingContent::try_from(request).unwrap();
        let content = MacContent::try_from(&content).unwrap().into();
        bob.receive_any_event(alice.user_id(), &content);

        let mut requests = bob.confirm().await.unwrap().0;
        assert!(requests.len() == 1);
        let request = requests.pop().unwrap();
        let content = OutgoingContent::try_from(request).unwrap();
        let content = MacContent::try_from(&content).unwrap().into();
        alice.receive_any_event(bob.user_id(), &content);

        assert!(alice.is_done());
        assert!(bob.is_done());
    }

    #[cfg(not(target_os = "macos"))]
    #[expect(clippy::unchecked_time_subtraction)]
    #[async_test]
    async fn test_timing_out() {
        use std::time::Duration;

        use ruma::time::Instant;

        let (alice_machine, bob) = setup_verification_machine().await;
        let alice = alice_machine.get_sas(bob.user_id(), bob.flow_id().as_str()).unwrap();

        assert!(!alice.timed_out());
        assert!(alice_machine.verifications.outgoing_requests().is_empty());

        // This line panics on macOS, so we're disabled for now.
        alice.set_creation_time(Instant::now() - Duration::from_secs(60 * 15));
        assert!(alice.timed_out());
        assert!(alice_machine.verifications.outgoing_requests().is_empty());
        alice_machine.garbage_collect();
        assert!(!alice_machine.verifications.outgoing_requests().is_empty());
        alice_machine.garbage_collect();
        assert!(alice_machine.verifications.is_empty());
    }

    /// Test to ensure that we cancel both verifications if a second one gets
    /// started while another one is going on.
    #[async_test]
    async fn test_double_verification_cancellation() {
        let (machine, bob_store) = verification_machine().await;

        let alice_device =
            bob_store.get_device(alice_id(), alice_device_id()).await.unwrap().unwrap();
        let identities = bob_store.get_identities(alice_device).await.unwrap();

        // Start the first sas verification.
        let (bob_sas, start_content) =
            Sas::start(identities.clone(), TransactionId::new(), true, None, None);

        machine
            .receive_any_event(&wrap_any_to_device_content(bob_sas.user_id(), start_content))
            .await
            .unwrap();

        let alice_sas = machine.get_sas(bob_sas.user_id(), bob_sas.flow_id().as_str()).unwrap();

        // We're not yet cancelled.
        assert!(!alice_sas.is_cancelled());

        let second_transaction_id = TransactionId::new();
        let (bob_sas, start_content) =
            Sas::start(identities, second_transaction_id.clone(), true, None, None);
        machine
            .receive_any_event(&wrap_any_to_device_content(bob_sas.user_id(), start_content))
            .await
            .unwrap();

        let second_sas = machine.get_sas(bob_sas.user_id(), bob_sas.flow_id().as_str()).unwrap();

        // Make sure we fetched the new one.
        assert_eq!(second_sas.flow_id().as_str(), second_transaction_id);

        // Make sure both of them are cancelled.
        assert!(alice_sas.is_cancelled());
        assert!(second_sas.is_cancelled());
    }

    /// Test to ensure that we cancel both verification requests if a second one
    /// gets started while another one is going on.
    #[async_test]
    async fn test_double_verification_request_cancellation() {
        let (machine, bob_store) = verification_machine().await;

        // Start the first verification request.
        let flow_id = FlowId::ToDevice("TEST_FLOW_ID".into());

        let bob_request = VerificationRequest::new(
            VerificationCache::new(),
            bob_store.clone(),
            flow_id.clone(),
            alice_id(),
            vec![],
            None,
        );

        let request = bob_request.request_to_device();
        let content: OutgoingContent = request.try_into().unwrap();

        machine
            .receive_any_event(&wrap_any_to_device_content(bob_request.own_user_id(), content))
            .await
            .unwrap();

        let alice_request =
            machine.get_request(bob_request.own_user_id(), bob_request.flow_id().as_str()).unwrap();

        // We're not yet cancelled.
        assert!(!alice_request.is_cancelled());

        let second_transaction_id = TransactionId::new();
        let bob_request = VerificationRequest::new(
            VerificationCache::new(),
            bob_store,
            second_transaction_id.clone().into(),
            alice_id(),
            vec![],
            None,
        );

        let request = bob_request.request_to_device();
        let content: OutgoingContent = request.try_into().unwrap();

        machine
            .receive_any_event(&wrap_any_to_device_content(bob_request.own_user_id(), content))
            .await
            .unwrap();

        let second_request =
            machine.get_request(bob_request.own_user_id(), bob_request.flow_id().as_str()).unwrap();

        // Make sure we fetched the new one.
        assert_eq!(second_request.flow_id().as_str(), second_transaction_id);

        // Make sure both of them are cancelled.
        assert!(alice_request.is_cancelled());
        assert!(second_request.is_cancelled());
    }

    /// Ensure that if a duplicate request is added (i.e. matching user and
    /// flow_id) the existing request is not cancelled and the new one is
    /// ignored
    #[async_test]
    async fn test_ignore_identical_verification_request() {
        let (machine, bob_store) = verification_machine().await;

        // Start the first verification request.
        let flow_id = FlowId::ToDevice("TEST_FLOW_ID".into());

        let bob_request = VerificationRequest::new(
            VerificationCache::new(),
            bob_store.clone(),
            flow_id.clone(),
            alice_id(),
            vec![],
            None,
        );

        let request = bob_request.request_to_device();
        let content: OutgoingContent = request.try_into().unwrap();

        machine
            .receive_any_event(&wrap_any_to_device_content(bob_request.own_user_id(), content))
            .await
            .unwrap();

        let first_request =
            machine.get_request(bob_request.own_user_id(), bob_request.flow_id().as_str()).unwrap();

        // We're not yet cancelled.
        assert!(!first_request.is_cancelled());

        // Bob is adding a second request with the same flow_id as before
        let bob_request = VerificationRequest::new(
            VerificationCache::new(),
            bob_store,
            flow_id.clone(),
            alice_id(),
            vec![],
            None,
        );

        let request = bob_request.request_to_device();
        let content: OutgoingContent = request.try_into().unwrap();

        machine
            .receive_any_event(&wrap_any_to_device_content(bob_request.own_user_id(), content))
            .await
            .unwrap();

        let second_request =
            machine.get_request(bob_request.own_user_id(), bob_request.flow_id().as_str()).unwrap();

        // None of the requests are cancelled
        assert!(!first_request.is_cancelled());
        assert!(!second_request.is_cancelled());
    }
}
