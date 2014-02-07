/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */


use std;
use capnp::capability::{FromClientHook};
use capnp::message::{MessageBuilder, MallocMessageBuilder, MessageReader};
use rpc_capnp::{Message, Return};
use rpc::{RpcConnectionState, RpcEvent, OutgoingMessage, Outgoing};

pub struct EzRpcClient {
    chan : std::comm::SharedChan<RpcEvent>,
}

impl EzRpcClient {
    pub fn new(server_address : &str) -> EzRpcClient {
        use std::io::net::{ip, tcp};

        let addr : ip::SocketAddr = FromStr::from_str(server_address).expect("bad server address");

        let mut tcp = tcp::TcpStream::connect(addr).unwrap();

        let connection_state = RpcConnectionState::new();

        let chan = connection_state.run(tcp.clone(), tcp);

        return EzRpcClient { chan : chan };
    }

    pub fn import_cap<T : FromClientHook>(&mut self, name : &str) -> T {
        let mut message = ~MallocMessageBuilder::new_default();
        let restore = message.init_root::<Message::Builder>().init_restore();
        restore.init_object_id().set_as_text(name);


        let (event, answer_port, question_port) = RpcEvent::new_outgoing(message);
        self.chan.send(event);

        let reader = answer_port.recv();
        let message = reader.get_root::<Message::Reader>();
        let client = match message.which() {
            Some(Message::Return(ret)) => {
                match ret.which() {
                    Some(Return::Results(payload)) => {
                        payload.get_content().get_as_capability::<T>()
                    }
                    _ => { fail!() }
                }
            }
            _ => {fail!()}
        };


        return client;

    }
}

pub struct EzRpcServer {
    chan : std::comm::SharedChan<RpcEvent>,
}