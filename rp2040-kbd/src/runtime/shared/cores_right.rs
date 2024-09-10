use crate::runtime::shared::loop_counter::LoopCount;
use core::sync::atomic::AtomicUsize;
use rp2040_hal::fugit::MicrosDurationU64;
use rp2040_kbd_lib::queue::{
    new_atomic_producer_consumer, AtomicQueueConsumer, AtomicQueueProducer,
};

#[derive(Debug, Copy, Clone)]
pub enum KeycoreToAdminMessage {
    Loop(LoopCount),
    Touch {
        tx_bytes: u16,
        loop_duration: MicrosDurationU64,
    },
    Reboot,
}

const QUEUE_CAPACITY: usize = 8;

pub type Producer = AtomicQueueProducer<'static, KeycoreToAdminMessage, QUEUE_CAPACITY>;

pub type Consumer = AtomicQueueConsumer<'static, KeycoreToAdminMessage, QUEUE_CAPACITY>;
static mut ATOMIC_QUEUE_MEM_AREA: [KeycoreToAdminMessage; QUEUE_CAPACITY] =
    [KeycoreToAdminMessage::Reboot; QUEUE_CAPACITY];
static mut ATOMIC_QUEUE_HEAD: AtomicUsize = AtomicUsize::new(0);
static mut ATOMIC_QUEUE_TAIL: AtomicUsize = AtomicUsize::new(0);
pub fn new_shared_queue() -> (Producer, Consumer) {
    #[expect(static_mut_refs)]
    unsafe {
        new_atomic_producer_consumer(
            &mut ATOMIC_QUEUE_MEM_AREA,
            &mut ATOMIC_QUEUE_HEAD,
            &mut ATOMIC_QUEUE_TAIL,
        )
    }
}

pub fn push_loop_to_admin(producer: &Producer, loop_count: LoopCount) -> bool {
    producer.push_back(KeycoreToAdminMessage::Loop(loop_count))
}

pub fn try_push_touch(
    producer: &Producer,
    transmitted: u16,
    loop_duration: MicrosDurationU64,
) -> bool {
    producer.push_back(KeycoreToAdminMessage::Touch {
        tx_bytes: transmitted,
        loop_duration,
    })
}

#[inline(never)]
pub fn push_reboot_and_halt(producer: &Producer) -> ! {
    while !producer.push_back(KeycoreToAdminMessage::Reboot) {}
    panic!("HALT AFTER PUSHING REBOOT");
}

pub fn pop_message(consumer: &Consumer) -> Option<KeycoreToAdminMessage> {
    consumer.pop_front()
}
