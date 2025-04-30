# Stockchain

So this project originated when I found a valid use case for Web3.
Putting Stock market transactions on a blockchain!
To prevent insider trading and stuff

But performance was my utmost importance, so I made a custom blockchain!
A perfect excuse to learn how the blockchain works under the hood and how a centralization based system (web2)
can be integrated with decentralization based system (web3)

# Working Research

https://chatgpt.com/share/6811084d-9ca4-800d-a4c6-7d69bcec6386

## Research Understanding

A client be a of 4 types

- Full Node
- Validator
- Attestor
- User

### Gossiping

Basically sharing info to peers

### Full Node

A user can opt into to be Full Node
Stores but doesn't gossip

- EVERY SINGLE BLOCK SINCE GENESIS, in order
  Stores and gossips when requested
- Account Balance
- Local Mempool

### Validator

A user can opt into to be Validator
Stores a copy of mempool
Randomly choosen when it comes around to create a new block.
Has ~10 seconds to collect transactions from mempool (usually picks from the top after sorting by gas per byte)
Package them together
Sign in and broadcast
Constructs a block including:

- Mempool transactions
- Reference to parent block
- Beacon chain data (validator registry, state root)
- Your own signature
- Randao reveal (randomness contribution for future validator selection)

They run the transactions on the current state and compute the next state hash as well

### Attestor

A user can opt into to be Attestor
You’re selected to vote on a proposed block.
You have a limited window (a few seconds) to:

- Download the block
- Check if it’s valid
- Sign your attestation
- Broadcast your vote

You run several checks locally before attesting:
Consensus and Data Checks

- Is the block’s parent valid and known?
- Is the block timestamp correct for the current slot?
- Is the proposer the correct one for this slot (as per the epoch schedule)?
- Are the signatures valid?
- Is the Randao reveal valid?

Execution Layer Checks

- Are all transactions valid (proper nonce, sufficient balance, correct gas)?
- Is the resulting state transition correct?
- Does the state root after applying the block match expectations?

These are cryptographic and deterministic checks — all attestors should independently reach the same conclusion.

### Validator and Attestor

To opt in, you have to deposit 32 ETH (this is specifically for ethermium obviously, but you get it, you have to stake)
Initiate Voluntary Exit:

- You send a `voluntary_exit` message from your validator client.
- This signals: “I want to leave the validator set and stop being selected.”

Exit Queue:

- You enter the exit queue, which helps protect the network from too many validators exiting at once.
- The queue length depends on how many others are exiting at the same time — could be instant, or several days.
- During this time, you're still expected to validate correctly (or risk penalties).

Withdrawal Delay:

- After your exit is processed, you enter a withdrawal period (currently ~1 day on average).
- Your stake (the original 32 ETH + earned rewards – penalties) becomes eligible for withdrawal.

# Roadmap
