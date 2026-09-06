`DeviceUpdates` now has a `deleted` field, listing the devices that the
homeserver stopped reporting in a `/keys/query` response. The updates delivered
when a device is logged out or removed were previously empty, so consumers
couldn't tell which device disappeared. Our own device is still never reported
as deleted.
