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

package org.matrix.rustcomponents.sdk.examples

import org.matrix.rustcomponents.sdk.Client
import org.matrix.rustcomponents.sdk.ClientBuilder
import org.matrix.rustcomponents.sdk.CrossSigningBootstrapOutcome
import org.matrix.rustcomponents.sdk.LogEvent
import org.matrix.rustcomponents.sdk.LogEventListener
import org.matrix.rustcomponents.sdk.LogLevel
import org.matrix.rustcomponents.sdk.PanicDetails
import org.matrix.rustcomponents.sdk.PanicListener
import org.matrix.rustcomponents.sdk.Room
import org.matrix.rustcomponents.sdk.Session
import org.matrix.rustcomponents.sdk.SyncService
import org.matrix.rustcomponents.sdk.TracingConfiguration
import org.matrix.rustcomponents.sdk.initPlatform
import org.matrix.rustcomponents.sdk.messageEventContentFromMarkdown
import org.matrix.rustcomponents.sdk.setLogEventListener
import org.matrix.rustcomponents.sdk.setPanicListener

/**
 * A walk-through of the FFI bindings: setting the platform up, building a
 * client, authenticating, syncing, sending a message and reading and writing
 * room state.
 *
 * Each step is a function of its own, so they can be read (and copied) in
 * isolation. [runExample] chains them in the order a client would.
 */
object Example {
    /**
     * Sets up logging and the async runtime.
     *
     * This has to happen once per process, before anything else in the SDK is
     * touched. Calling it twice throws.
     */
    fun setUpPlatform(logDirectory: String? = null) {
        initPlatform(
            config = TracingConfiguration(
                logLevel = LogLevel.INFO,
                traceLogPacks = emptyList(),
                extraTargets = emptyList(),
                // Logcat on Android, stderr elsewhere.
                writeToStdoutOrSystem = true,
                // Pass a `TracingFileConfiguration` to also write rotated log
                // files under `logDirectory`.
                writeToFiles = null,
            ),
            // `true` sets up a smaller runtime, for memory-constrained
            // processes such as the iOS notification service extension.
            useLightweightTokioRuntime = false,
        )

        // Optional: receive the SDK's log events, for instance to forward them
        // to the platform logger.
        setLogEventListener(object : LogEventListener {
            override fun onLogEvent(event: LogEvent) {
                println("[${event.level}] ${event.target}: ${event.message}")
            }
        })

        // Optional: learn about panics, so the application can offer a crash
        // report rather than only seeing a failed call.
        setPanicListener(object : PanicListener {
            override fun onPanic(details: PanicDetails) {
                println("The SDK panicked: ${details.message} at ${details.file}:${details.line}")
            }
        })
    }

    /**
     * Builds a client for the homeserver hosting [userId].
     *
     * `sessionPaths` is where the SDK keeps its state and its caches. Both
     * directories must exist, and must be unique per session: a second client
     * pointed at the same paths fails to open the store.
     */
    suspend fun buildClient(userId: String, dataPath: String, cachePath: String): Client =
        ClientBuilder()
            .sessionPaths(dataPath = dataPath, cachePath = cachePath)
            // Resolves the homeserver through the user's server name. Use
            // `homeserverUrl` when the URL is already known.
            .serverNameFromUserId(userId = userId)
            .userAgent(userAgent = "matrix-rust-sdk-example/1.0")
            .build()

    /**
     * Logs in with a password and returns the session to persist.
     *
     * Store the session somewhere durable, and encrypted: it holds the access
     * token. [restore] turns it back into a usable client.
     */
    suspend fun logIn(client: Client, username: String, password: String): Session {
        client.login(
            username = username,
            password = password,
            initialDeviceName = "Example client",
            deviceId = null,
        )

        return client.session()
    }

    /**
     * Restores a client from a previously stored session, skipping the login.
     *
     * The client has to be built with the same `sessionPaths` the session was
     * created with.
     */
    suspend fun restore(client: Client, session: Session) {
        client.restoreSession(session = session)
    }

    /**
     * Creates a cross-signing identity for the account, if it has none.
     *
     * The homeserver usually asks the user to authenticate first, in which
     * case the outcome carries the challenge to answer; call this again with
     * the matching `AuthData`, session included.
     */
    suspend fun setUpEncryption(client: Client) {
        when (val outcome = client.encryption().bootstrapCrossSigning(authData = null)) {
            is CrossSigningBootstrapOutcome.Bootstrapped ->
                println("Created a cross-signing identity")
            is CrossSigningBootstrapOutcome.AlreadyBootstrapped ->
                println("The account already has a cross-signing identity")
            is CrossSigningBootstrapOutcome.AuthenticationRequired ->
                println("Authenticate first, then try again: ${outcome.challenge.flows}")
            is CrossSigningBootstrapOutcome.Unavailable ->
                println("Cross-signing is unavailable: ${outcome.message}")
        }
    }

    /**
     * Starts syncing and returns the service, which must be kept alive for as
     * long as the client is used. Call `stop` on it when shutting down.
     */
    suspend fun startSyncing(client: Client): SyncService {
        val syncService = client.syncService().finish()
        syncService.start()
        return syncService
    }

    /** Lists the rooms the account has joined. */
    fun listRooms(client: Client) {
        for (room in client.rooms()) {
            println("${room.id()}: ${room.displayName() ?: "(unnamed)"}")
        }
    }

    /**
     * Sends a message to a room.
     *
     * Messages go through the room's timeline, which also queues them while
     * the device is offline and retries them later.
     */
    suspend fun sendMessage(room: Room, markdown: String) {
        val timeline = room.timeline()
        timeline.send(msg = messageEventContentFromMarkdown(md = markdown))
    }

    /**
     * A tour of the basic room operations: reading and writing state, room
     * account data, and membership.
     */
    suspend fun roomOperations(client: Client, roomId: String) {
        // Rooms the client knows about are looked up by ID; `joinRoomByIdOrAlias`
        // joins one it does not.
        val room = client.getRoom(roomId = roomId) ?: client.joinRoomById(roomId = roomId)

        println("Topic: ${room.topic()}")
        room.setTopic(topic = "Set from the example")

        // Any state event can be read as raw JSON, by type and state key.
        val joinRules = room.stateEvent(eventType = "m.room.join_rules", stateKey = "")
        println("Join rules: $joinRules")

        // As can every state event of a type, for instance the memberships.
        println("${room.stateEvents(eventType = "m.room.member").size} member events")

        // Room account data is per-user and per-room, and is not sent to
        // anyone else.
        room.setAccountData(
            eventType = "com.example.notes",
            content = """{"note":"written by the example"}""",
        )
        println("Notes: ${room.accountData(eventType = "com.example.notes")}")

        // Global account data works the same way, off the client.
        println("Direct rooms: ${client.accountData(eventType = "m.direct")}")

        room.inviteUserById(userId = "@friend:example.org")
    }

    /**
     * The whole flow, in the order a client would run it.
     *
     * Pass the room ID of a room the account has joined.
     */
    suspend fun runExample(
        userId: String,
        password: String,
        roomId: String,
        dataPath: String,
        cachePath: String,
    ) {
        setUpPlatform()

        val client = buildClient(userId = userId, dataPath = dataPath, cachePath = cachePath)
        val session = logIn(client = client, username = userId, password = password)
        println("Logged in as ${session.userId} on ${session.deviceId}")

        setUpEncryption(client = client)

        val syncService = startSyncing(client = client)
        try {
            listRooms(client = client)

            val room = client.getRoom(roomId = roomId) ?: error("Not a joined room: $roomId")
            sendMessage(room = room, markdown = "Hello from the **Rust SDK**")

            roomOperations(client = client, roomId = roomId)
        } finally {
            syncService.stop()
        }
    }
}
