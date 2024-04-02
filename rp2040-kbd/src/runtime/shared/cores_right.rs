use crate::runtime::locks::CrossCoreMsgLock;
use crate::runtime::shared::loop_counter::LoopCount;
use crate::runtime::shared::ring_buffer::RingBuffer;

#[derive(Debug, Copy, Clone)]
pub enum KeycoreToAdminMessage {
    Touch,
    Loop(LoopCount),
}

static mut SHARED_KEY_CORE_TO_ADMIN: RingBuffer<16, KeycoreToAdminMessage> = RingBuffer::new();

pub fn push_touch_to_admin() -> bool {
    let _guard = CrossCoreMsgLock::claim();
    // Safety: Exclusive access through lock
    unsafe { SHARED_KEY_CORE_TO_ADMIN.try_push(KeycoreToAdminMessage::Touch) }
}

pub fn push_loop_to_admin(loop_count: LoopCount) -> bool {
    let _guard = CrossCoreMsgLock::claim();
    unsafe { SHARED_KEY_CORE_TO_ADMIN.try_push(KeycoreToAdminMessage::Loop(loop_count)) }
}

pub fn pop_message() -> Option<KeycoreToAdminMessage> {
    let _guard = CrossCoreMsgLock::claim();
    // Safety: Exclusive access through lock
    unsafe { SHARED_KEY_CORE_TO_ADMIN.try_pop() }
}
