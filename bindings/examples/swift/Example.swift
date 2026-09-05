// Copyright 2025 The Matrix.org Foundation C.I.C.
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

import Foundation
import MatrixRustSDK

/// A walk-through of the FFI bindings: setting the platform up, building a
/// client, authenticating, syncing, sending a message and reading and writing
/// room state.
///
/// Each step is a function of its own, so they can be read (and copied) in
/// isolation. `runExample` chains them in the order a client would.
enum Example {
    /// Sets up logging and the async runtime.
    ///
    /// This has to happen once per process, before anything else in the SDK is
    /// touched. Calling it twice throws.
    static func setUpPlatform() throws {
        try initPlatform(config: TracingConfiguration(logLevel: .info,
                                                      traceLogPacks: [],
                                                      extraTargets: [],
                                                      // The system log on Apple platforms.
                                                      writeToStdoutOrSystem: true,
                                                      // Pass a `TracingFileConfiguration` to also
                                                      // write rotated log files.
                                                      writeToFiles: nil),
                         // `true` sets up a smaller runtime, for memory-constrained
                         // processes such as the notification service extension.
                         useLightweightTokioRuntime: false)

        // Optional: receive the SDK's log events, for instance to forward them
        // to the platform logger.
        setLogEventListener(listenerToSet: LogPrinter())

        // Optional: learn about panics, so the application can offer a crash
        // report rather than only seeing a failed call.
        setPanicListener(listenerToSet: PanicPrinter())
    }

    /// Builds a client for the homeserver hosting `userID`.
    ///
    /// `sessionPaths` is where the SDK keeps its state and its caches. Both
    /// directories must exist, and must be unique per session: a second client
    /// pointed at the same paths fails to open the store.
    static func buildClient(userID: String, dataPath: String, cachePath: String) async throws -> Client {
        try await ClientBuilder()
            .sessionPaths(dataPath: dataPath, cachePath: cachePath)
            // Resolves the homeserver through the user's server name. Use
            // `homeserverUrl` when the URL is already known.
            .serverNameFromUserId(userId: userID)
            .userAgent(userAgent: "matrix-rust-sdk-example/1.0")
            .build()
    }

    /// Logs in with a password and returns the session to persist.
    ///
    /// Store the session somewhere durable, and encrypted: it holds the access
    /// token. `restore` turns it back into a usable client.
    static func logIn(client: Client, username: String, password: String) async throws -> Session {
        try await client.login(username: username,
                               password: password,
                               initialDeviceName: "Example client",
                               deviceId: nil)

        return client.session()
    }

    /// Restores a client from a previously stored session, skipping the login.
    ///
    /// The client has to be built with the same `sessionPaths` the session was
    /// created with.
    static func restore(client: Client, session: Session) async throws {
        try await client.restoreSession(session: session)
    }

    /// Creates a cross-signing identity for the account, if it has none.
    ///
    /// The homeserver usually asks the user to authenticate first, in which
    /// case the outcome carries the challenge to answer; call this again with
    /// the matching `AuthData`, session included.
    static func setUpEncryption(client: Client) async throws {
        switch try await client.encryption().bootstrapCrossSigning(authData: nil) {
        case .bootstrapped:
            print("Created a cross-signing identity")
        case .alreadyBootstrapped:
            print("The account already has a cross-signing identity")
        case .authenticationRequired(let challenge):
            print("Authenticate first, then try again: \(challenge.flows)")
        case .unavailable(let message):
            print("Cross-signing is unavailable: \(message)")
        }
    }

    /// Starts syncing and returns the service, which must be kept alive for as
    /// long as the client is used. Call `stop` on it when shutting down.
    static func startSyncing(client: Client) async throws -> SyncService {
        let syncService = try await client.syncService().finish()
        await syncService.start()
        return syncService
    }

    /// Lists the rooms the account has joined.
    static func listRooms(client: Client) {
        for room in client.rooms() {
            print("\(room.id()): \(room.displayName() ?? "(unnamed)")")
        }
    }

    /// Sends a message to a room.
    ///
    /// Messages go through the room's timeline, which also queues them while
    /// the device is offline and retries them later.
    static func sendMessage(room: Room, markdown: String) async throws {
        let timeline = try await room.timeline()
        _ = try await timeline.send(msg: messageEventContentFromMarkdown(md: markdown))
    }

    /// A tour of the basic room operations: reading and writing state, room
    /// account data, and membership.
    static func roomOperations(client: Client, roomID: String) async throws {
        // Rooms the client knows about are looked up by ID; `joinRoomByIdOrAlias`
        // joins one it does not.
        let room = if let room = client.getRoom(roomId: roomID) {
            room
        } else {
            try await client.joinRoomById(roomId: roomID)
        }

        print("Topic: \(room.topic() ?? "(none)")")
        try await room.setTopic(topic: "Set from the example")

        // Any state event can be read as raw JSON, by type and state key.
        let joinRules = try await room.getStateEvent(eventType: "m.room.join_rules", stateKey: "")
        print("Join rules: \(joinRules ?? "(none)")")

        // As can every state event of a type, for instance the memberships.
        let members = try await room.getStateEvents(eventType: "m.room.member")
        print("\(members.count) member events")

        // Room account data is per-user and per-room, and is not sent to
        // anyone else.
        try await room.setAccountData(eventType: "com.example.notes",
                                      content: #"{"note":"written by the example"}"#)
        let notes = try await room.accountData(eventType: "com.example.notes")
        print("Notes: \(notes ?? "(none)")")

        // Global account data works the same way, off the client.
        let directRooms = try await client.accountData(eventType: "m.direct")
        print("Direct rooms: \(directRooms ?? "(none)")")

        try await room.inviteUserById(userId: "@friend:example.org")
    }

    /// The whole flow, in the order a client would run it.
    ///
    /// Pass the room ID of a room the account has joined.
    static func runExample(userID: String,
                           password: String,
                           roomID: String,
                           dataPath: String,
                           cachePath: String) async throws {
        try setUpPlatform()

        let client = try await buildClient(userID: userID, dataPath: dataPath, cachePath: cachePath)
        let session = try await logIn(client: client, username: userID, password: password)
        print("Logged in as \(session.userId) on \(session.deviceId)")

        try await setUpEncryption(client: client)

        let syncService = try await startSyncing(client: client)
        defer { Task { await syncService.stop() } }

        listRooms(client: client)

        guard let room = client.getRoom(roomId: roomID) else {
            fatalError("Not a joined room: \(roomID)")
        }
        try await sendMessage(room: room, markdown: "Hello from the **Rust SDK**")

        try await roomOperations(client: client, roomID: roomID)
    }
}

/// Prints the SDK's log events. A real client would forward them to its own
/// logger instead.
final class LogPrinter: LogEventListener {
    func onLogEvent(event: LogEvent) {
        print("[\(event.level)] \(event.target): \(event.message)")
    }
}

/// Reports panics raised inside the SDK.
final class PanicPrinter: PanicListener {
    func onPanic(details: PanicDetails) {
        print("The SDK panicked: \(details.message) at \(details.file ?? "?"):\(details.line ?? 0)")
    }
}
