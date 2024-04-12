use crate::runtime::locks::CrossCoreMsgLock;
use crate::runtime::shared::loop_counter::LoopCount;
use rp2040_kbd_lib::ring_buffer::RingBuffer;

#[derive(Debug, Copy, Clone)]
pub enum KeycoreToAdminMessage {
    Loop(LoopCount),
    Tx(u16),
    Reboot,
}

static mut SHARED_KEY_CORE_TO_ADMIN: RingBuffer<KeycoreToAdminMessage, 16> = RingBuffer::new();

pub fn push_loop_to_admin(loop_count: LoopCount) -> bool {
    let _guard = CrossCoreMsgLock::claim();
    unsafe { SHARED_KEY_CORE_TO_ADMIN.try_push(KeycoreToAdminMessage::Loop(loop_count)) }
}

pub fn try_push_tx(transmitted: u16) -> bool {
    if let Some(_guard) = CrossCoreMsgLock::try_claim() {
        unsafe { SHARED_KEY_CORE_TO_ADMIN.try_push(KeycoreToAdminMessage::Tx(transmitted)) }
    } else {
        false
    }
}

#[inline(never)]
pub fn push_reboot_and_halt() -> ! {
    loop {
        let Some(_guard) = CrossCoreMsgLock::try_claim() else {
            continue;
        };
        unsafe {
            if !SHARED_KEY_CORE_TO_ADMIN.try_push(KeycoreToAdminMessage::Reboot) {
                continue;
            }
            break;
        }
    }
    panic!("HALT AFTER PUSHING REBOOT");
}

pub fn pop_message() -> Option<KeycoreToAdminMessage> {
    let _guard = CrossCoreMsgLock::claim();
    // Safety: Exclusive access through lock
    unsafe { SHARED_KEY_CORE_TO_ADMIN.try_pop() }
}
