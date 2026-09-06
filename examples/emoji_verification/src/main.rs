#![recursion_limit = "256"]

use std::io::Write;

use anyhow::{Context, Result};
use clap::Parser;
use futures_util::stream::StreamExt;
use matrix_sdk::{
    Client,
    config::SyncSettings,
    encryption::verification::{
        Emoji, SasState, SasVerification, Verification, VerificationRequest,
        VerificationRequestState, format_emojis,
    },
    ruma::{
        OwnedUserId, UserId,
        events::{
            key::verification::request::ToDeviceKeyVerificationRequestEvent,
            room::message::{MessageType, OriginalSyncRoomMessageEvent},
        },
    },
};
use url::Url;

async fn wait_for_confirmation(sas: SasVerification, emoji: [Emoji; 7]) {
    println!("\nDo the emojis match: \n{}", format_emojis(emoji));
    print!("Confirm with `yes` or cancel with `no`: ");
    std::io::stdout().flush().expect("We should be able to flush stdout");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("error: unable to read user input");

    match input.trim().to_lowercase().as_ref() {
        "yes" | "true" | "ok" => sas.confirm().await.unwrap(),
        _ => sas.cancel().await.unwrap(),
    }
}

async fn print_devices(user_id: &UserId, client: &Client) {
    println!("Devices of user {user_id}");

    for device in client.encryption().get_user_devices(user_id).await.unwrap().devices() {
        if device.device_id()
            == client.device_id().expect("We should be logged in now and know our device id")
        {
            continue;
        }

        println!(
            "   {:<10} {:<30} {:<}",
            device.device_id(),
            device.display_name().unwrap_or("-"),
            if device.is_verified() { "✅" } else { "❌" }
        );
    }
}

/// Drive a SAS verification to its end, whichever side started it.
async fn sas_verification_handler(client: Client, sas: SasVerification) {
    println!(
        "Starting verification with {} {}",
        sas.other_device().user_id(),
        sas.other_device().device_id()
    );
    print_devices(sas.other_device().user_id(), &client).await;
    sas.accept().await.unwrap();

    let mut stream = sas.changes();

    while let Some(state) = stream.next().await {
        match state {
            SasState::KeysExchanged { emojis, decimals: _ } => {
                tokio::spawn(wait_for_confirmation(
                    sas.clone(),
                    emojis.expect("We only support verifications using emojis").emojis,
                ));
            }
            SasState::Done { .. } => {
                let device = sas.other_device();

                println!(
                    "Successfully verified device {} {} {:?}",
                    device.user_id(),
                    device.device_id(),
                    device.local_trust_state()
                );

                print_devices(sas.other_device().user_id(), &client).await;

                break;
            }
            SasState::Cancelled(cancel_info) => {
                println!("The verification has been cancelled, reason: {}", cancel_info.reason());

                break;
            }
            SasState::Created { .. }
            | SasState::Started { .. }
            | SasState::Accepted { .. }
            | SasState::Confirmed => (),
        }
    }
}

/// Follow a verification request until it turns into a concrete verification
/// flow, or ends.
///
/// The same loop works for a request we received and for one we sent: the only
/// difference is that an incoming request has to be accepted first, which is
/// what `accept` selects.
async fn verification_request_handler(client: Client, request: VerificationRequest, accept: bool) {
    if accept {
        println!("Accepting verification request from {}", request.other_user_id());
        request.accept().await.expect("Can't accept verification request");
    } else {
        println!("Waiting for {} to accept the verification request", request.other_user_id());
    }

    let mut stream = request.changes();

    while let Some(state) = stream.next().await {
        match state {
            VerificationRequestState::Created { .. }
            | VerificationRequestState::Requested { .. } => (),
            VerificationRequestState::Ready { .. } => {
                // The side that sent the request picks the method, once the other side
                // has told us which ones it supports. Starting SAS moves the request
                // into the `Transitioned` state, which is where the flow is picked up
                // below, for both sides.
                if !accept {
                    request.start_sas().await.expect("Can't start a SAS verification");
                }
            }
            VerificationRequestState::Transitioned { verification } => {
                // We only support SAS verification.
                if let Verification::SasV1(s) = verification {
                    tokio::spawn(sas_verification_handler(client, s));
                    break;
                }
            }
            VerificationRequestState::Done { other_device_data } => {
                match other_device_data {
                    Some(device) => println!(
                        "Verification with {} {} is done",
                        device.user_id(),
                        device.device_id()
                    ),
                    None => println!("The verification request was handled by another device"),
                }

                break;
            }
            VerificationRequestState::Cancelled(cancel_info) => {
                println!("The verification was cancelled, reason: {}", cancel_info.reason());
                break;
            }
        }
    }
}

/// Register the handlers for verification requests that reach us
/// asynchronously, either as a to-device event or as a message in a room.
fn add_verification_request_handlers(client: &Client) {
    client.add_event_handler(
        |ev: ToDeviceKeyVerificationRequestEvent, client: Client| async move {
            let request = client
                .encryption()
                .get_verification_request(&ev.sender, &ev.content.transaction_id)
                .await
                .expect("Request object wasn't created");

            tokio::spawn(verification_request_handler(client, request, true));
        },
    );

    client.add_event_handler(|ev: OriginalSyncRoomMessageEvent, client: Client| async move {
        if let MessageType::VerificationRequest(_) = &ev.content.msgtype {
            let request = client
                .encryption()
                .get_verification_request(&ev.sender, &ev.event_id)
                .await
                .expect("Request object wasn't created");

            tokio::spawn(verification_request_handler(client, request, true));
        }
    });
}

/// Send a verification request to another user (or to our own other devices)
/// and drive it.
///
/// The identity of the user we want to verify only exists locally once we have
/// downloaded their device keys, which is why this runs after a first sync
/// rather than straight after login.
async fn request_verification(client: &Client, user_id: &UserId) -> Result<()> {
    let identity = client
        .encryption()
        .get_user_identity(user_id)
        .await?
        .context("We don't know about that user's cross-signing identity")?;

    let request = identity.request_verification().await?;

    println!("Sent a verification request to {user_id}");

    tokio::spawn(verification_request_handler(client.clone(), request, false));

    Ok(())
}

async fn sync(client: Client, verify: Option<OwnedUserId>) -> Result<()> {
    add_verification_request_handlers(&client);

    // A first sync so that we know about the other side's devices, and so that a
    // request they sent before we started shows up.
    let response = client.sync_once(SyncSettings::new()).await?;

    if let Some(user_id) = verify {
        request_verification(&client, &user_id).await?;
    }

    client.sync(SyncSettings::new().token(response.next_batch)).await?;

    Ok(())
}

#[derive(Parser, Debug)]
struct Cli {
    /// The homeserver to connect to.
    #[clap(value_parser)]
    homeserver: Url,

    /// The user name that should be used for the login.
    #[clap(value_parser)]
    user_name: String,

    /// The password that should be used for the login.
    #[clap(value_parser)]
    password: String,

    /// Send a verification request to this user instead of only waiting for
    /// one.
    ///
    /// Pass our own user ID to verify another one of our own devices.
    #[clap(short, long)]
    verify: Option<OwnedUserId>,

    /// Set the proxy that should be used for the connection.
    #[clap(short, long)]
    proxy: Option<Url>,

    /// The user to send a verification request to.
    ///
    /// Use your own user ID to verify another one of your devices. If this is
    /// left out, we only wait for somebody else to start a verification.
    #[clap(short = 'u', long)]
    verify_user: Option<OwnedUserId>,

    /// Enable verbose logging output.
    #[clap(long, action)]
    verbose: bool,
}

async fn login(
    homeserver: Url,
    proxy: Option<Url>,
    user_name: &str,
    password: &str,
) -> Result<Client> {
    let builder = Client::builder().homeserver_url(homeserver);

    let builder = if let Some(proxy) = proxy { builder.proxy(proxy) } else { builder };

    let client = builder.build().await?;

    client
        .matrix_auth()
        .login_username(user_name, password)
        .initial_device_display_name("rust-sdk")
        .await?;

    Ok(client)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt::init();
    }

    let client = login(cli.homeserver, cli.proxy, &cli.user_name, &cli.password).await?;

    sync(client, cli.verify).await?;

    Ok(())
}
