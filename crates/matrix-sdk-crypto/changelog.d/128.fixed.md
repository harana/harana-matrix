`Device::set_local_trust` now applies the trust change to the device as the
store currently holds it, under a lock, instead of writing back the whole
in-memory copy it was called on. Fields changed elsewhere in the meantime
(`deleted`, `olm_wedging_index`, `withheld_code_sent`) are no longer reverted.
