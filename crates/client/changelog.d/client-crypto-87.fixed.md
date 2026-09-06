Olm session timestamps read out of a pickle are now clamped to the current time
if they lie in the future. A pickle written by another implementation could
carry a `creation_time` in milliseconds rather than seconds, which made that
session sort ahead of every other session forever and get picked for unwedging.
