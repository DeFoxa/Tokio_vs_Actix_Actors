Architecture Notes (MatchingEngine framework):
    - top level MANAGER:
        - Manager spawns the server_client actors (workers) -> server_client connect to third party communication servers (json-rpc, Rest, WebSocket), number, type and parameter
        of server_client spawned specified from the instantiator in fn main() -> Manager also generates the required ports for server_client event handling.

        - Manager spawns Sequencer (connects to server_client ports via Require<Port>) and receives events/messages containing local types -> handles sequence of ob_update
        stream and trade stream assuring interaction between order_book state and trade stream are sequentially ordered for the matching engine. Similar to actix/tokio actor
        models, the sequencer will contain handling for dropped data connections, or mis-ordered incoming data streams, pausing passing to matching engine if ob state update
        is down for any reason. At a later date Will add a backup Rest server client for cases when the WebSocket stream is down: this will also eventually be integrated into
        the ob_state system for faster state refresh due to the 250ms time limit between ob_partial_depth data push.

        - Lastly, Manager spawns the matching engine actors and connects via direct message passing from the sequencer (no port spawn), ME contains handling for matching algo, 
        taker flow on orderbook liquidity and finalization on executed trades

    Worker Actors:
        - ServerClient: takes inputs from the manager and open connections to the external servers, deserialize incoming data to a local type and pass the generated 
        events to either  a port (most likely) or directly send the messages/events to the next actor in the chain.
        - Sequencer: described above, number spawned relative to number of exchange connections
        - MatchingEngine: described above, number spawned relative to number of exchange connections
   Event Connections:
        - Deserialized data Port: Provide on server client, Require on sequencer: Indication = deserialized type, Request = Never
        - Sequenced Data: Direct messages sent from sequencer to ME. Can also spawn a port on sequencer for DB write actor or any lower performance actors requiring access to
        sequenced data

Redesign Notes:

Server-Client(external data sources): moving these processes to another program to run within tokio async/channel concurrency framework. This will allow more robut testing of aeron, simplifies the kompact concurrency setup and will allow using receive_network event handling within kompact (which I'm interested in testing/building)

General Framework:
    Serverclient program connects to external communication servers(WS, Rest, RPC)
    Deserializes to server specific type 
        -> passes events through aeron producer 
            -> kompact receive_network consumes incoming data, received by data normalizer within kompact, converts to normalized type
                -> normalized types sent to sequencer
                    -> sent by sequencer to data processing/modeling component
                        -> internal state change messages to active components
                            -> outgoing requests sent (to client sytsem or to external servers directly) 
                            



