use crate::keymap::KeymapLayer;
use crate::runtime::shared::loop_counter::LoopCount;
use core::sync::atomic::AtomicUsize;
use rp2040_hal::fugit::MicrosDurationU64;
use rp2040_kbd_lib::queue::{
    new_atomic_producer_consumer, AtomicQueueConsumer, AtomicQueueProducer,
};

#[derive(Debug, Copy, Clone)]
pub enum KeycoreToAdminMessage {
    // Notify on any user action
    TouchLeft(MicrosDurationU64),
    TouchRight(MicrosDurationU64),
    // Send loop count to calculate scan latency
    Loop(LoopCount),
    // Output which layer is active
    LayerChange(KeymapLayer),
    // Output bytes received over UART
    Rx(u16),
    // Write a boot message then trigger usb-boot
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

pub fn push_touch_left_to_admin(
    atomic_queue_producer: &Producer,
    duration: MicrosDurationU64,
) -> bool {
    atomic_queue_producer.push_back(KeycoreToAdminMessage::TouchLeft(duration))
}

pub fn push_touch_right_to_admin(
    atomic_queue_producer: &Producer,
    duration: MicrosDurationU64,
) -> bool {
    atomic_queue_producer.push_back(KeycoreToAdminMessage::TouchRight(duration))
}

pub fn push_loop_to_admin(atomic_queue_producer: &Producer, loop_count: LoopCount) -> bool {
    atomic_queue_producer.push_back(KeycoreToAdminMessage::Loop(loop_count))
}

pub fn push_layer_change(atomic_queue_producer: &Producer, new_layer: KeymapLayer) -> bool {
    atomic_queue_producer.push_back(KeycoreToAdminMessage::LayerChange(new_layer))
}

pub fn push_rx_change(atomic_queue_producer: &Producer, received: u16) -> bool {
    atomic_queue_producer.push_back(KeycoreToAdminMessage::Rx(received))
}

#[inline(never)]
pub fn push_reboot_and_halt(atomic_queue_producer: &Producer) -> ! {
    while !atomic_queue_producer.push_back(KeycoreToAdminMessage::Reboot) {}
    panic!("HALT AFTER PUSHING REBOOT");
}

pub fn pop_message(atomic_queue_consumer: &Consumer) -> Option<KeycoreToAdminMessage> {
    atomic_queue_consumer.pop_front()
}
