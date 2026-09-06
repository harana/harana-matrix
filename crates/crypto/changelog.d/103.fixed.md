A device can now be unwedged when the wedged Olm session is the first one we
ever created for it. `mark_device_as_wedged` used to give up when it found no
stored session to check the rate limit against, leaving no way to recover.
