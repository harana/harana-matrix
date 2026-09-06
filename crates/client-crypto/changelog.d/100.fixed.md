When several Olm sessions exist with a device, the one that most recently
decrypted a message from that device is now used to encrypt, as the spec
recommends, instead of the most recently created one. Sessions that have never
decrypted anything still fall back to creation order.
