`Client::create_room` with `is_space` set now goes through
`Client::create_space`, so the created space also gets the power levels that
keep ordinary members from posting into the space room itself.
