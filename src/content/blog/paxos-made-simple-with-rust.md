---
title: "Writing Simple Paxos Consensus Algorithm With Rust"
pubDate: 2024-02-12T11:55:28.536Z
draft: false
tags:
  - distsys
  - rust
---

## What Is Consensus?

I would say that one of the core properties of a system is the agreement on which data
are right at a given moment.

At that moment, one of the more important aspects of a distributed system shows up:
the consensus. The consensus in that system will ensure that every participant agrees
on a single state, even if someone fails.

Consensus plays an important role in systems that need to bring data consistency
around the current state. Some use cases where consensus brings a strong advantage
are network agreement in a server cluster, database replications, and even in blockchain
networks.

## The Simple Paxos

The Simple Paxos algorithm is the most simple implementation of the Paxos family
algorithms. As presented by Leslie Lamport on the [The Part-Time Parliament](https://lamport.azurewebsites.net/pubs/lamport-paxos.pdf)
and [Paxos Made Simple](https://lamport.azurewebsites.net/pubs/paxos-simple.pdf),
the idea of the Simple Paxos is simple:

1. Your node can assume 3 different roles: _proposers_, _acceptors_ and _learners_.
2. Your node can be all these roles at the same time.
3. An acceptor will accept the first value that is received as a proposal.
4. A _proposer_ will receive a promise from every _acceptor_, that promise let
   the _proposer_ knows that the node will accept the proposed value.
5. A _proposer_ reach the _quorum_, that is, the majority, when receive a promise
   of $\frac{N}{2} + 1$ where $N$ is the number of nodes in the system.
6. After reach the _quorum_, the _learners_ will receive the accepted value from
   the _acceptors_. The _learners_ will put the accepted value into their state
   machines.

![Paxos Sequence Diagram](/assets/paxos-sequence-diagram.png)

(This is a simple sequence diagram coming from Wikipedia, but it's a good representation)

### Preparing the proposal for a new value

A _proposer_ is the node that will propose a new value to reach the consensus
around their system. In this case, each proposal follows this pattern: `(id, v)`,
where the `id` is an incrementally natural number, and `v` is the proposed value,
for our implementation, we will assume `v` as a string.

First things first. We need to start our server. In this case, using Rust, I will
use Axum to do all the work for me:

```rust
use Axum::{
    routing::{get, post},
    Router,
    http::StatusCode,
    extract::{State, Json}
};
use clap::Parser;
use serde::{Serialize, Deserialize};
use tokio::sync::Mutex;

type Ledger = HashMap<Id, Value>;
struct AppState {
    node: Node,
    nodes: Arc<Mutex<Vec<Node>>>,
    acceptor: Arc<Mutex<Acceptor>>,
    proposer: Arc<Mutex<Proposer>>,
    ledger: Arc<Mutex<Ledger>>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    id: u64,
    #[arg(short, long)]
    port: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let port = args.port;
    let node_id = args.id;

    let node_http_addr = format!("0.0.0.0:{}", port);

    println!("Starting new node: http://{}", node_http_addr);

    let node = Node::new(node_id, node_http_addr.parse().unwrap());
    let state = AppState {
        node,
        nodes: Arc::new(Mutex::new(Vec::new())),
        acceptor: Arc::new(Mutex::new(Acceptor::default())),
        proposer: Arc::new(Mutex::new(Proposer::new())),
        ledger: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/prepare", post(prepare))
        .route("/handle-prepare", post(handle_prepare))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(node_http_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Clone, Debug)]
struct Proposer {
    pub id: u64,
}

impl Proposer {
    pub fn new() -> Self {
        Self {
            id: 0,
        }
    }

    pub async fn prepare(&mut self, state: &AppState, value: String) -> Result<Proposal, String> {
      todo!()
    }

    pub async fn propose(&self, state: &AppState, propose: &Proposal) -> Result<(), String> {
      todo!()
    }
}

#[derive(Clone, Debug, Default)]
struct Acceptor {
    pub last_proposal_number: u64,
    pub accepted_proposal: Option<Propose>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Proposal {
    pub id: u64,
    pub value: Option<String>,
}

async fn prepare() -> () {
  todo!()
}

async fn handle_prepare() -> () {
  todo!()
}
```

Now, we have a server running on the port that we want. You can test it by running:

```bash
cargo run -- --id 1 --port 3000
```

We will implement the proposer logic on the `/prepare` endpoint. The _proposer_
will send a prepare message to all _acceptor_ messages and wait
for the promises.

```rust
async fn prepare(State(state): State<AppState>, value: String) -> (StatusCode, String) {
    let mut proposer = state.proposer.lock().await;
    let proposal = match proposer.prepare(&state, value).await {
        Err(e) => return (StatusCode::BAD_REQUEST, e.clone()),
        Ok(proposal) => proposal,
    };

    todo!()
}
```

In this first phase, we will only send the prepare message to all acceptors. The
prepare message is a simple message with the `id` of the proposal. The acceptor will
receive this message and will send a promise to the proposer.

```rust
impl Proposer {
    // ...

    pub async fn prepare(&mut self, state: &AppState, value: String) -> Result<Proposal, String> {
        let client = Client::new();

        self.id += 1;

        let nodes = state.nodes.lock().await;
        let reqs = nodes.iter().map(|node| {
            client.post(format!("http://{}/handle-prepare", node.addr))
            .json(&self.id)
            .send()
        });

        let responses = futures::future::join_all(reqs).await;

        let mut promises = Vec::with_capacity(responses.len());

        for response in responses.into_iter().flatten() {
            promises.push(response.json::<HandleProposalPayload>().await.unwrap());
        }

        let quorum = (nodes.len() / 2) + 1;

        if promises.len() < quorum {
            return Err(String::from("Proposal does not receive promises of the entire quorum"));
        }

        let accepted_promise = promises
            .into_iter()
            .filter(|promise| promise.value.is_some())
            .max_by_key(|promise| match &promise.value {
                None => 0,
                Some(value) => value.id,
            });

        // Sorry, I'm not proud of it, but it's the best that I can do for now. :(
        // Feel free to improve it (PLEASE!).
        let value = match accepted_promise {
            None => Some(value),
            Some(payload) => {
                match payload.value {
                    None => Some(value),
                    Some(proposal) => {
                        match proposal.value {
                            None => Some(value),
                            b => b,
                        }
                    },
                }
            }
        };

        let proposal = Proposal { id: self.id, value };

        Ok(proposal)
    }
}
```

We implement the `proposer.prepare` method to send the prepare message to all acceptors. What
this is doing is:

- Calling the `/handle-prepare` endpoint of all nodes in the system.
- Waiting for the promises of the acceptors.
- Checking if the proposer received the quorum of promises.
- If the proposer received the quorum, it will check if there is a value that was accepted
  by the acceptors. If there is, the proposer will use this value as the proposed value. If
  not, the proposer will use the value that it wants to propose.
- Return the proposal.

### Receiving the prepared value and sending the promise

Cool. We're coming to what we can call the phase `1b`. The _acceptor_ will receive
the prepare message and need to send a promise if everything is OK. In this case, which
things do we need to check?

- If the proposal number is greater than the last proposal number that the acceptor
  received.
- If the acceptor already accepted a value, if yes, it needs to send the accepted
  value to the _proposer_.

```rust
#[derive(Serialize, Deserialize, Debug)]
struct HandleProposalPayload {
    error: Option<String>,
    value: Option<Ballot>,
}

async fn handle_prepare(State(state): State<AppState>, proposal_id: String) -> (StatusCode, Json<HandleProposalPayload>) {
    let proposal_id: u64 = proposal_id.parse().unwrap();
    let mut acceptor = state.acceptor.lock().await;

    if proposal_id <= acceptor.last_proposal_number {
        let payload = HandleProposalPayload {
            error: Some(String::from("The proposal ID is lesser than the last accepted proposal number")),
            value: None,
        };
        return (StatusCode::BAD_REQUEST, Json(payload));
    }

    if acceptor.accepted_proposal.is_some() && acceptor.last_proposal_number == proposal_id {
        let value = acceptor.accepted_proposal.clone();
        let payload = HandleProposalPayload {
            error: None,
            value: Some(Proposal { id: proposal_id, value }),
        };
        return (StatusCode::OK, Json(payload));
    }

    acceptor.last_proposal_number = proposal_id;

    let payload = HandleProposalPayload {
        error: None,
        value: Some(Proposal { id: proposal_id, value: None }),
    };

    (StatusCode::OK, Json(payload))
}
```

We improved the `handle-prepare` function to handle all the logic of the acceptor
receiving the prepare message, instead of just a `todo!()`. If everything is OK,
the acceptor will return a promise to the proposer. If not, it will return an error
message.

### Receiving the promise and handling the acceptance message

Now that the proposer received the promise, we need to start with the phase `2a`.
The proposer will send the accept message to all acceptors. The accept message needs
to contain the proposal and their identifier.

Then, the first part of our implementation will be improving the implementation of our
`Proposer` struct. In this case, we will need to add a new function called `propose`.

```rust
#[derive(Serialize, Deserialize, Debug)]
struct HandleAcceptPayload {
    error: Option<String>,
    value: Option<Ballot>,
}

impl Proposer {
    // ...

    pub async fn propose(&self, state: &AppState, proposal: &Proposal) -> Result<(), String> {
        let client = Client::new();
        let nodes = state.nodes.lock().await;

        let reqs = nodes.iter().map(|node| {
            client.post(format!("http://{}/handle-accept", node.addr))
                .json(&proposal)
                .send()
        });

        let responses = futures::future::join_all(reqs).await;

        let mut accepted_promises = Vec::with_capacity(responses.len());

        for response in responses.into_iter().flatten() {
            accepted_promises.push(response.json::<HandleAcceptPayload>().await.unwrap());
        }

        let quorum = (nodes.len() / 2) + 1;

        if accepted_promises.len() + 1 < quorum {
            return Err(String::from("Proposal not accepted by majority"));
        }

        Ok(())
    }
}
```

This `propose` function will send the accept message to all acceptors and wait for the
acceptance of the proposal. If the proposer receives the quorum of acceptances, it will
return `Ok(())`. If not, it will return an error message.

Now, we need to implement the `/handle-accept` endpoint to handle the accept message.

```rust
async fn handle_accept(State(state): State<AppState>, propose: Json<Ballot>) -> (StatusCode, Json<HandleAcceptPayload>) {
    let mut acceptor = state.acceptor.lock().await;
    if acceptor.last_ballot_number != propose.id {
        let payload = HandleAcceptPayload {
            error: Some(String::from("Node received a proposal with a ballot ID different!")),
            value: None,
        };
        return (StatusCode::BAD_REQUEST, Json(payload));
    }

    acceptor.accepted_proposal = propose.value.clone();

    let payload = HandleAcceptPayload {
        error: None,
        value: Some(Ballot { id: propose.id, value: propose.value.clone() }),
    };

    (StatusCode::OK, Json(payload))
}
```

Now, we have the acceptor receiving the accept message and handling it. If everything
is OK, the acceptor will accept the proposal and return an acceptance message to the
proposer.

We will need to add the `proposer.propose` call on the `/prepare` endpoint.

```rust
async fn prepare(State(state): State<AppState>, value: String) -> (StatusCode, String) {
    // ...

    match proposer.propose(&state, &ballot).await {
        Err(e) => (StatusCode::BAD_REQUEST, e),
        Ok(_) => {
            let client = Client::new();
            let nodes = state.nodes.lock().await;

            let reqs = nodes.iter().map(|node| {
                client.post(format!("http://{}/handle-learn", node.addr))
                    .json(&ballot)
                    .send()
            });

            futures::future::join_all(reqs).await;

            (StatusCode::OK, String::from("Proposal accepted by the majority!"))
        },
    }
}
```

Removing that old `todo!()` and putting the `proposer.propose` call on the `/prepare`,
we have the proposer sending the accept message to all acceptors and waiting for
the acceptance of the proposal. If the proposer receives the quorum of acceptances,
it will send the accepted value to all learners.

### Learning the accepted value

The last part of our implementation is the `/handle-learn` endpoint. The learner
will receive the accepted value and put it into their state machines.

```rust
async fn handle_learn(State(state): State<AppState>, payload: Json<Ballot>) -> (StatusCode, ()) {
    let mut ledger = state.ledger.lock().await;
    match &payload.value {
        None => ledger.insert(payload.id, String::from("")),
        Some(v) => ledger.insert(payload.id, v.clone()),
    };
    (StatusCode::OK, ())
}
```

After learning the accepted value, the learner will put it into their state machines. And
then we have the consensus around the system. The _acceptor_ will
send the accepted value to the _learner_, but I think it's easier to implement
just sending the accepted value to the _learner_ via _proposer_.

## Conclusion

This is the most simple implementation of a consensus algorithm. You have some
variants that bring some improvements to the Paxos flow, like the Multi-Paxos
and EPaxos. All of these have some properties that are best for specific
use cases.

If you want to see the repo with the implementation of this algorithm, you can
access it [here](https://github.com/noghartt/paxos-from-scratch).

Thanks to [hnrbs](https://github/hnrbs), one of my inspirations to write this
code and learn more about Paxos and consensus.
You can see [his implementation here](https://github.com/hnrbs/paxos).

## Further Reading

- [The Part-Time Parliament](https://lamport.azurewebsites.net/pubs/lamport-paxos.pdf)
- [Paxos Made Simple](https://lamport.azurewebsites.net/pubs/paxos-simple.pdf)
- Database Internals
- Designing Data-Intensive Applications
