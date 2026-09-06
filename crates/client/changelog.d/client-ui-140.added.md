`UtdHookManager::with_late_decryption_grace_period` suppresses the UTD report
entirely for an event that gets decrypted within the given period of being
marked as a UTD, matching Web's behaviour of ignoring decryptions that land
within about four seconds. Without it, such events were still reported, with
`time_to_decrypt` set.
