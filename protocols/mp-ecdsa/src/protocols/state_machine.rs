use crate::util::DebugLogger;
use multi_party_ecdsa::gg_2020::state_machine::traits::RoundBlame;
use multi_party_ecdsa::MessageRoundID;
use round_based::{Msg, StateMachine};
use sp_application_crypto::serde::Serialize;
use std::collections::HashSet;
use std::fmt::Debug;

pub(crate) struct StateMachineWrapper<T> {
    sm: T,
    current_round_blame: tokio::sync::watch::Sender<CurrentRoundBlame>,
    // stores a list of received messages
    received_messages: HashSet<Vec<u8>>,
    logger: DebugLogger,
}

impl<T: StateMachine + RoundBlame + Debug> StateMachineWrapper<T> {
    pub fn new(
        sm: T,
        current_round_blame: tokio::sync::watch::Sender<CurrentRoundBlame>,
        logger: DebugLogger,
    ) -> Self {
        Self {
            sm,
            current_round_blame,
            logger,
            received_messages: HashSet::new(),
        }
    }
}

impl<T> StateMachine for StateMachineWrapper<T>
where
    T: StateMachine + RoundBlame + Debug,
    <T as StateMachine>::Err: Debug,
    <T as StateMachine>::MessageBody: Serialize + MessageRoundID,
{
    type Err = T::Err;
    type Output = T::Output;
    type MessageBody = T::MessageBody;

    fn handle_incoming(&mut self, msg: Msg<Self::MessageBody>) -> Result<(), Self::Err> {
        let (round, sender, _receiver) = (msg.body.round_id(), msg.sender, msg.receiver);

        self.logger.trace(format!(
            "Handling incoming message round={}, sender={}",
            round, sender
        ));

        if round < self.current_round().into() {
            self.logger
                .trace(format!("Message for round={round} is outdated, ignoring",));
            return Ok(());
        }

        // Before passing to the state machine, make sure that we haven't already received the same
        // message (this is needed as we use a gossiping protocol to send messages, and we don't
        // want to process the same message twice)
        let msg_serde = bincode2::serialize(&msg).expect("Failed to serialize message");
        if !self.received_messages.insert(msg_serde) {
            self.logger.trace(format!(
                "Already received message for round={}, sender={}",
                round, sender
            ));
            return Ok(());
        }

        let result = self.sm.handle_incoming(msg.clone());

        if let Some(err) = result.as_ref().err() {
            self.logger.error(format!("StateMachine error: {err:?}"));
        }

        // Get the round blame to update round blame
        let round_blame = self.round_blame();

        self.logger.trace(format!(
            "SM After: {:?} || round_blame: {:?}",
            &self.sm, round_blame
        ));

        result
    }

    fn message_queue(&mut self) -> &mut Vec<Msg<Self::MessageBody>> {
        self.sm.message_queue()
    }

    fn wants_to_proceed(&self) -> bool {
        self.sm.wants_to_proceed()
    }

    fn proceed(&mut self) -> Result<(), Self::Err> {
        self.logger.trace(format!(
            "Trying to proceed: current round ({:?}), waiting for msgs from parties: ({:?})",
            self.current_round(),
            self.round_blame(),
        ));
        let result = self.sm.proceed();

        self.logger.trace(format!(
            "Proceeded through SM. New current round ({:?}), waiting for msgs from parties: ({:?})",
            self.current_round(),
            self.round_blame(),
        ));

        result
    }

    fn round_timeout(&self) -> Option<std::time::Duration> {
        self.sm.round_timeout()
    }

    fn round_timeout_reached(&mut self) -> Self::Err {
        self.sm.round_timeout_reached()
    }

    fn is_finished(&self) -> bool {
        self.sm.is_finished()
    }

    fn pick_output(&mut self) -> Option<Result<Self::Output, Self::Err>> {
        self.sm.pick_output()
    }

    fn current_round(&self) -> u16 {
        self.sm.current_round()
    }

    fn total_rounds(&self) -> Option<u16> {
        self.sm.total_rounds()
    }

    fn party_ind(&self) -> u16 {
        self.sm.party_ind()
    }

    fn parties(&self) -> u16 {
        self.sm.parties()
    }
}

impl<T: StateMachine + RoundBlame> RoundBlame for StateMachineWrapper<T> {
    fn round_blame(&self) -> (u16, Vec<u16>) {
        let (unreceived_messages, blamed_parties) = self.sm.round_blame();
        self.logger
            .debug(format!("Not received messages from : {blamed_parties:?}"));
        let _ = self.current_round_blame.send(CurrentRoundBlame {
            unreceived_messages,
            blamed_parties: blamed_parties.clone(),
        });
        (unreceived_messages, blamed_parties)
    }
}

#[derive(Default, Debug)]
pub struct CurrentRoundBlame {
    pub unreceived_messages: u16,
    pub blamed_parties: Vec<u16>,
}
