use core::fmt::Write;

pub(crate) struct DebugBuffer {
    inner: [u8; 4096 * 4],
    offset: usize,
}

impl Write for DebugBuffer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let buf = s.as_bytes();
        let mut avail = &mut self.inner[self.offset..];
        if avail.len() >= buf.len() {
            avail[self.offset..self.offset + buf.len()].copy_from_slice(buf);
            self.offset += buf.len();
        }
        Ok(())
    }
}

impl DebugBuffer {
    pub(crate) const fn new() -> Self {
        Self {
            inner: [0u8; 4096 * 4],
            offset: 0,
        }
    }

    #[inline]
    pub(crate) fn use_content<T, F: FnOnce(&[u8]) -> T>(&mut self, func: F) {
        func(&self.inner[..self.offset]);
        self.offset = 0;
    }
}
