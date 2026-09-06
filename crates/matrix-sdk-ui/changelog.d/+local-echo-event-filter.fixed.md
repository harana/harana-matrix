Local echoes go through the timeline's event filter, like remote events do. A
timeline built with a custom filter used to show the send queue's local echoes
for events the filter excludes, so the same event was visible while it was being
sent and gone once it came back from the server. As for a remote event, the
filter only decides whether the echo becomes a timeline item of its own: an edit
or a reaction is still applied to the item it targets.
