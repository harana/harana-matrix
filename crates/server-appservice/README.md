# server-appservice

Application service registration handling: the namespace regexes an appservice
registration declares, and the questions a homeserver, bridge, or bot asks of
them.

- [`NamespaceRegex`] compiles a registration's namespaces into exclusive and
  non-exclusive regex sets.
- [`RegistrationInfo`] pairs a `Registration` with those compiled sets and the
  sender user ID derived from its localpart, and answers whether a user, alias,
  or room falls in the appservice's namespaces.
- [`Registrations`] holds the loaded registrations and answers the same
  questions across all of them, plus lookup by the tokens an appservice
  authenticates with.

Users are matched per [MSC3905]: an appservice's `users` namespace claims local
users only, so a remote user whose ID happens to match the regex is not claimed.

Ported from [tuwunel]'s `src/service/appservice`, minus its storage and HTTP
transaction handling.

[MSC3905]: https://github.com/matrix-org/matrix-spec-proposals/pull/3905
[tuwunel]: https://github.com/matrix-construct/tuwunel
