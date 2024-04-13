use crate::runtime::shared::loop_counter::LoopCount;
use rp2040_kbd_lib::queue::{AtomicQueueConsumer, AtomicQueueProducer};

#[derive(Debug, Copy, Clone)]
pub enum KeycoreToAdminMessage {
    Loop(LoopCount),
    Tx(u16),
    Reboot,
}

pub type Producer = AtomicQueueProducer<'static, KeycoreToAdminMessage, 32>;

pub fn push_loop_to_admin(producer: &Producer, loop_count: LoopCount) -> bool {
    producer.push_back(KeycoreToAdminMessage::Loop(loop_count))
}

pub fn try_push_tx(producer: &Producer, transmitted: u16) -> bool {
    producer.push_back(KeycoreToAdminMessage::Tx(transmitted))
}

#[inline(never)]
pub fn push_reboot_and_halt(producer: &Producer) -> ! {
    while !producer.push_back(KeycoreToAdminMessage::Reboot) {}
    panic!("HALT AFTER PUSHING REBOOT");
}

pub fn pop_message(
    consumer: &AtomicQueueConsumer<'static, KeycoreToAdminMessage, 32>,
) -> Option<KeycoreToAdminMessage> {
    consumer.pop_front()
}
