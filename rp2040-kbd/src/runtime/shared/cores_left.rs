use crate::keymap::KeymapLayer;
use crate::runtime::shared::loop_counter::LoopCount;
use rp2040_hal::fugit::MicrosDurationU64;
use rp2040_kbd_lib::queue::{AtomicQueueConsumer, AtomicQueueProducer};

#[derive(Debug, Copy, Clone)]
pub enum KeycoreToAdminMessage {
    // Notify on any user action
    TouchLeft(MicrosDurationU64),
    TouchRight(MicrosDurationU64),
    // Send loop count to calculate scan latency
    Loop(LoopCount),
    // Output which layer is active
    LayerChange(KeymapLayer, Option<KeymapLayer>),
    // Output bytes received over UART
    Rx(u16),
    // Write a boot message then trigger usb-boot
    Reboot,
}

pub type Producer = AtomicQueueProducer<'static, KeycoreToAdminMessage, 32>;

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

pub fn push_layer_change(
    atomic_queue_producer: &Producer,
    new_layer: KeymapLayer,
    new_tmp: Option<KeymapLayer>,
) -> bool {
    atomic_queue_producer.push_back(KeycoreToAdminMessage::LayerChange(new_layer, new_tmp))
}

pub fn push_rx_change(atomic_queue_producer: &Producer, received: u16) -> bool {
    atomic_queue_producer.push_back(KeycoreToAdminMessage::Rx(received))
}

#[inline(never)]
pub fn push_reboot_and_halt(atomic_queue_producer: &Producer) -> ! {
    while !atomic_queue_producer.push_back(KeycoreToAdminMessage::Reboot) {}
    panic!("HALT AFTER PUSHING REBOOT");
}

pub fn pop_message(
    atomic_queue_consumer: &AtomicQueueConsumer<'static, KeycoreToAdminMessage, 32>,
) -> Option<KeycoreToAdminMessage> {
    atomic_queue_consumer.pop_front()
}
