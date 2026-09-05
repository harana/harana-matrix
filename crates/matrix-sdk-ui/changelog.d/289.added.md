`NotificationItem` gained `can_send_message`, computed from the room's power
levels for `m.room.message`. It lets a client decide whether to offer a direct
reply action on a notification without loading the room's power levels itself.
