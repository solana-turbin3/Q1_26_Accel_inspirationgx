use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy)]
pub struct Contributor {
    pub amount: [u8; 8],
}

impl Contributor {
    pub const LEN: usize = core::mem::size_of::<Contributor>();

    pub fn to_bytes(&self) -> &[u8; Self::LEN] {
        bytemuck::bytes_of(self).try_into().unwrap()
    }
}
