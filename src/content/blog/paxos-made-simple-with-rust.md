---
title: "Writing Simple Paxos Consensus Algorith With Rust"
pubDate: 2024-02-12T11:55:28.536Z
draft: true
tags:
  - distsys
---

## What Is Consensus?

I would say that one of the core properties of a system is the agreement of which data
are really right at a given moment.

At that moment, one of the more important aspects of a distributed systems shows up:
the consensus. The consensus in that system will ensure that every participant agree
on a single state, even if someone fails.

Consensus does an important role on systems that needs to bring data consistency
around the current state. Some use cases where consensus brings a strong advantage
are network agreement in a server cluster, database replications and even in blockchain
networks.

## The Simple Paxos

The Simple Paxos algorithm is the most simple implementation of the Paxos family
algorithms. As presented by Leslie Lamport on the [The Part-Time Parliament](https://lamport.azurewebsites.net/pubs/lamport-paxos.pdf)
and [Paxos Made Simple](https://lamport.azurewebsites.net/pubs/paxos-simple.pdf),
the idea of the Simple Paxos is simple:

1. Your node can assume 3 different roles: _proposers_, _acceptors_ and _learners_.
2. Your node can be all these roles at the same time.
3. An acceptor will accept the first value that receive as a proposal.
4. A _proposer_ will receive a promise from every _acceptor_, that promise let
   the _proposer_ knows that the node will accept the proposed value.
5. A _proposer_ reach the _quorum_, that is, the majority, when receive a promise
   of $\frac{N}{2} + 1$ where $N$ is the number of nodes in the system.
6. After reach the _quorum_, the _learners_ will receive the accepted value from
   the _acceptors_. The _learners_ will put the accepted value into their state
   machines.

### Preparing the proposal of a new value

A _proposer_ is the node that will propose a new value for reach the consensus
around their system. In case, each proposal follows this pattern: `(id, v)`,
where the `id` is a incrementally natural number, and `v` is the proposed value,
for our implementation, we will assume `v` as a string.

First things first. We need to start our server. In case, using Rust, I will
use Axum to do all the work for me:

```rust
use axum::{
  routing::{get, post},
    Router,
    http::StatusCode,
    extract::{State, Json}
};
use clap::Parser;
use serde::{Serialize, Deserialize};
use tokio::sync::Mutex;

struct AppState {
    node: Node,
    nodes: Arc<Mutex<Vec<Node>>>,
    acceptor: Arc<Mutex<Acceptor>>,
    proposer: Arc<Mutex<Proposer>>,
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

Now, we have a server running on the port that we want. ou can test it by running:

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

### Accepting a proposal and sending a promise

### Receiving the promise and sending the accepted value

### Learning the accepted value

## Conclusion

This is the most simple implementation of a consensus algorithm. You have some
variants that brings some improvements on the Paxos flow, like the Multi-Paxos
and EPaxos. All of these ones have some properties that are best for specific
use cases.

If you want to see the repo with the implementation of this algorithm, you can
access it [here](https://github.com/noghartt/paxos-from-scratch).

## Further Reading

- [The Part-Time Parliament](https://lamport.azurewebsites.net/pubs/lamport-paxos.pdf)
- [Paxos Made Simple](https://lamport.azurewebsites.net/pubs/paxos-simple.pdf)
- Database Internals
- Designing Data-Intensive Applications
