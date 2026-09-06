Clearing a timeline that has local echoes now keeps the aggregations that
involve them. They were dropped along with the remote ones, which lost a
reaction or an edit made while a reset was in flight and, worse, lost the
mapping from a transaction id to its aggregation, leaving it stuck in its local
state once the send queue reported it sent.
