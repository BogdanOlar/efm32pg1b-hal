//! Linked Direct Memory Access
//!
//! # ChannelTransfer
//!
//! Memory-to-memory transfer
//!
//! **WARNING**: May panic if the `ChannelTransfer` is dropped while the DMA channel is still active. Make sure you
//!              `resolve()` the transfer before exiting the scope of the transfer.
//!
//! Example:
//!
//! ```rust,no_run
//!    let p = pac::Peripherals::take().unwrap();
//!
//!    let dma = Dma::init(p.ldma);
//!    let ch = dma.ch1;
//!
//!    const SRC_U8: [u8; BUF_UNIT_SIZE] = {
//!        let mut seq = [0; BUF_UNIT_SIZE];
//!        let mut i = 0;
//!        // Fill the buffer with values from 1 to 255
//!        while i < BUF_UNIT_SIZE {
//!            seq[i] = 1 + (i % u8::MAX as usize) as u8;
//!            i += 1;
//!        }
//!        seq
//!    };
//!    let src = &SRC_U8[0..];
//!    let mut dst_u8: [u8; BUF_UNIT_SIZE] = [0u8; _];
//!
//!    // take an unaligned slice of `dst` so that the transfer is done with bytes, not half-words or words for this
//!    // example
//!    let dst = &mut dst_u8[1..TRANSFER_UNIT_COUNT + 1];
//!
//!    info!("src: {} bytes @ 0x{:X}", src.len(), src.as_ptr().addr());
//!    info!("dst: {} bytes @ 0x{:X}", dst.len(), dst.as_ptr().addr());
//!
//!    let transfer = ch.try_into_transfer(&SRC_U8, dst).unwrap();
//!
//!    let result_token = loop {
//!        match transfer.check_done() {
//!            Some(token) => break token,
//!            None => {
//!                info!(".")
//!            }
//!        }
//!    };
//!
//!    let res = transfer.resolve(result_token);
//!    info!("Result: {}", res);
//!
//! ```

use crate::pac::{
    ldma::ch::ctrl::{BLOCKSIZE, DSTINC, SIZE, SRCINC, STRUCTTYPE},
    Ldma,
};
use core::cmp::min;
use cortex_m::asm;

/// DMA driver
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Dma {
    /// DMA channel 0
    pub ch0: DmaChannel,
    /// DMA channel 1
    pub ch1: DmaChannel,
    /// DMA channel 2
    pub ch2: DmaChannel,
    /// DMA channel 3
    pub ch3: DmaChannel,
    /// DMA channel 4
    pub ch4: DmaChannel,
    /// DMA channel 5
    pub ch5: DmaChannel,
    /// DMA channel 6
    pub ch6: DmaChannel,
    /// DMA channel 7
    pub ch7: DmaChannel,
}

impl Dma {
    /// Initialize DMA
    pub fn init(_dma_p: Ldma) -> Self {
        // Enable DMA clock
        let cmu = unsafe { crate::pac::Cmu::steal() };
        cmu.hfbusclken0().modify(|_, w| w.ldma().set_bit());

        Self {
            ch0: DmaChannel { id: ChannelId::Ch0 },
            ch1: DmaChannel { id: ChannelId::Ch1 },
            ch2: DmaChannel { id: ChannelId::Ch2 },
            ch3: DmaChannel { id: ChannelId::Ch3 },
            ch4: DmaChannel { id: ChannelId::Ch4 },
            ch5: DmaChannel { id: ChannelId::Ch5 },
            ch6: DmaChannel { id: ChannelId::Ch6 },
            ch7: DmaChannel { id: ChannelId::Ch7 },
        }
    }
}

/// DMA channel singleton
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DmaChannel {
    /// Channel ID
    id: ChannelId,
}

impl DmaChannel {
    /// Get the DMA channel ID
    pub fn id(&self) -> ChannelId {
        self.id
    }

    /// Get channel enabled
    pub fn enabled(&self) -> bool {
        mmio::ch_enabled(self.id)
    }

    /// Get channel busy (if enabled)
    pub fn busy(&self) -> bool {
        self.enabled() && mmio::ch_busy(self.id)
    }

    /// Start a memory-to-memory transfer
    ///
    /// Fails if the length of `src` or `dest` is `0`
    pub fn try_into_transfer<'a, W: Sized>(
        self,
        src: &'a [W],
        dst: &'a mut [W],
    ) -> Result<ChannelTransfer<'a, W>, DmaError> {
        let total_byte_cnt = min(core::mem::size_of_val(src), core::mem::size_of_val(dst));

        if total_byte_cnt == 0 {
            return Err(DmaError::InvalidTransferSize(self));
        }

        Ok(ChannelTransfer::new(self, src, dst, total_byte_cnt))
    }
}

/// DMA channel identifier
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ChannelId {
    /// Channel 0
    Ch0,
    /// Channel 1
    Ch1,
    /// Channel 2
    Ch2,
    /// Channel 3
    Ch3,
    /// Channel 4
    Ch4,
    /// Channel 5
    Ch5,
    /// Channel 6
    Ch6,
    /// Channel 7
    Ch7,
}

/// DMA channel specialised for memory-to-memory transfer
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ChannelTransfer<'a, W: Sized> {
    id: ChannelId,
    src: &'a [W],
    dst: Option<&'a mut [W]>,
    byte_count: usize,
    unit: SIZE,
}

impl<'a, W: Sized> ChannelTransfer<'a, W> {
    fn new(ch: DmaChannel, src: &'a [W], dst: &'a mut [W], byte_count: usize) -> Self {
        // Decide which unit/type the transfer may use
        let unit = if src.as_ptr().addr().is_multiple_of(size_of::<u32>())
            && dst.as_ptr().addr().is_multiple_of(size_of::<u32>())
            && dst.len().is_multiple_of(size_of::<u32>())
        {
            SIZE::Word
        } else if src.as_ptr().addr().is_multiple_of(size_of::<u16>())
            && dst.as_ptr().addr().is_multiple_of(size_of::<u16>())
            && dst.len().is_multiple_of(size_of::<u16>())
        {
            SIZE::Halfword
        } else {
            SIZE::Byte
        };

        let mut transfer = Self {
            id: ch.id,
            src,
            dst: Some(dst),
            byte_count,
            unit,
        };

        transfer.start();

        transfer
    }

    /// Start the DMA transfer
    fn start(&mut self) {
        let dst: &mut [u8] = unsafe {
            core::slice::from_raw_parts_mut(
                self.dst.as_mut().unwrap().as_ptr() as *mut u8,
                self.byte_count,
            )
        };

        assert_eq!(self.byte_count, core::mem::size_of_val(dst));
        assert_ne!(dst.len(), 0);

        let unit_byte_size = 1 << self.unit as u8;
        let total_units = dst.len() / unit_byte_size;
        assert_eq!(dst.len() % unit_byte_size, 0);

        mmio::ch_enable_clear(self.id);
        mmio::ien_clear(self.id);
        mmio::if_clear(self.id);

        let arr_end = dst[dst.len()..].as_ptr().addr();
        let aligned_end_addr = arr_end - (arr_end % align_of::<SerializedDescriptor>());

        let last_descr_addr = aligned_end_addr - size_of::<SerializedDescriptor>();
        let last_chunk_min_units = (arr_end - last_descr_addr) / unit_byte_size;
        assert_eq!((arr_end - last_descr_addr) % unit_byte_size, 0);

        let descr_count = if total_units.is_multiple_of(Descriptor::MAX_TRANSFER_UNITS) {
            total_units / Descriptor::MAX_TRANSFER_UNITS
        } else {
            (total_units / Descriptor::MAX_TRANSFER_UNITS) + 1
        };
        // First descriptor will be written to DMA channel register, not to the descriptor list
        let linked_list_count = descr_count - 1;
        let linked_list_start_addr =
            aligned_end_addr - (linked_list_count * size_of::<SerializedDescriptor>());
        // Create the descriptor list at the end of the destination buffer
        let descriptor_list = unsafe {
            core::slice::from_raw_parts_mut(
                linked_list_start_addr as *mut SerializedDescriptor,
                linked_list_count,
            )
        };

        let mut remaining_units = total_units;

        // Create first descriptor
        let first_descr_units = if remaining_units > Descriptor::MAX_TRANSFER_UNITS {
            min(
                Descriptor::MAX_TRANSFER_UNITS,
                remaining_units - last_chunk_min_units,
            )
        } else {
            remaining_units
        };
        remaining_units -= first_descr_units;
        let first_descriptor = Descriptor {
            ctrl: Ctrl::new()
                .with_size(self.unit)
                .with_struct_req()
                .with_block_size(BLOCKSIZE::All)
                .with_xfer_cnt(first_descr_units.try_into().unwrap()),
            src: self.src.as_ptr().addr(),
            dst: dst.as_ptr().addr(),
            link: if remaining_units > 0 {
                Link::new().with_absolute_addr(descriptor_list.as_ptr().addr())
            } else {
                Link::default()
            },
        };

        // Fill in the linked descriptors
        for (i, ser_descr) in descriptor_list.iter_mut().enumerate() {
            let is_last = i == (linked_list_count - 1);

            let descr_units = if is_last {
                remaining_units
            } else {
                min(
                    Descriptor::MAX_TRANSFER_UNITS,
                    remaining_units - last_chunk_min_units,
                )
            };
            assert!(descr_units <= Descriptor::MAX_TRANSFER_UNITS);

            let addr_offset = (total_units - remaining_units) * unit_byte_size;

            let descr = Descriptor {
                ctrl: Ctrl::new()
                    .with_size(self.unit)
                    .with_struct_req()
                    .with_block_size(BLOCKSIZE::All)
                    .with_xfer_cnt(descr_units.try_into().unwrap()),
                src: self.src.as_ptr().addr() + addr_offset,
                dst: dst.as_ptr().addr() + addr_offset,
                link: if !is_last {
                    Link::new().with_relative_addr(1)
                } else {
                    Link::default()
                },
            };

            *ser_descr = descr.into();

            remaining_units -= descr_units;
        }
        assert_eq!(remaining_units, 0);

        // make sure all linked descriptors have been written before proceeding
        asm::dsb();

        // First descriptor is always written directly to the DMA peripheral in order to support transfers smaller than
        // the size of a descriptor (in which case we don't use descriptor linked list)
        mmio::ch_write_descriptor(self.id, &first_descriptor);

        // start the transfer
        mmio::ch_done_clear(self.id);
        mmio::ch_enable_set(self.id);
        mmio::ch_start(self.id);
    }

    /// Check if DMA transfer is done, and obtain a `ResolvedToken` if it is.
    ///
    /// Example:
    ///
    /// ```rust,no_run
    /// let transfer = ch.try_into_transfer(&src, &mut dst).unwrap();
    /// let result_token: ResovledToken = loop {
    ///     match transfer.check_done() {
    ///         Some(token) => break token,
    ///         None => {
    ///             info!(".")
    ///         }
    ///     }
    /// };
    /// let res: Result<(DmaChannel, usize), DmaChannel> = transfer.resolve(result_token);
    /// ```
    pub fn check_done(&self) -> Option<ResovledToken> {
        if mmio::ch_error(self.id) {
            Some(ResovledToken {
                result: TransferResult::Err,
            })
        } else if mmio::ch_done(self.id) {
            Some(ResovledToken {
                result: TransferResult::Ok,
            })
        } else {
            None
        }
    }

    /// Resolve the transfer
    ///
    /// See [`ChannelTransfer::check_done()`] for how to obtain the `ResovledToken`
    pub fn resolve(self, token: ResovledToken) -> Result<(DmaChannel, usize), DmaError> {
        mmio::ien_clear(self.id);
        mmio::if_clear(self.id);
        mmio::ch_enable_clear(self.id);
        mmio::ch_done_clear(self.id);

        // Make sure channel is disabled, since Self is going to get dropped, and will panic if the channel is enabled
        asm::dsb();

        match token.result {
            TransferResult::Ok => Ok((DmaChannel { id: self.id }, self.byte_count)),
            TransferResult::Err => Err(DmaError::Transfer(DmaChannel { id: self.id })),
        }
    }

    /// Get the Unit size that the transfer is using: byte, half-word (16 bits), word (32 bits)
    ///
    /// The unit size is calculated dynamically when the Transfer is created, based on the alignment an length of
    /// `self.src` and `self.dst`, and it favors the widest bitwidth (word--32 bits), followed by half-word, followed
    /// by byte size
    pub fn unit(&self) -> SIZE {
        self.unit
    }
}

impl<'a, W: Sized> Drop for ChannelTransfer<'a, W> {
    fn drop(&mut self) {
        if mmio::ch_enabled(self.id) {
            panic!("`ChannelTransfer` was dropped while DMA channel was still active");
        }
    }
}

/// Transfer result token
///
/// Needs to be passed to `ChannelTransfer::resolve()` as proof that the transfer resolved (either sucessfuly, or with
/// an error), in order to resolve the transfer
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ResovledToken {
    result: TransferResult,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum TransferResult {
    Ok,
    Err,
}

/// DMA Error
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DmaError {
    /// Invalid transfer size (e.g. transfer size may not be 0)
    InvalidTransferSize(DmaChannel),
    /// DMA transfer failed
    Transfer(DmaChannel),
}

/// DMA Channel Descriptor
///
/// Can be written to the DMA peripheral to trigger a transfer, or converted to a `SerializedDescriptor` to create
/// descriptor linked lists
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) struct Descriptor {
    ctrl: Ctrl,
    src: usize,
    dst: usize,
    link: Link,
}

impl Descriptor {
    /// Maximum number of units (byte, half-word, word) which can be transfered in one DMA shot
    #[cfg(not(feature = "dma-debug-max-transfer"))]
    const MAX_TRANSFER_UNITS: usize = 1 << 12;
    #[cfg(feature = "dma-debug-max-transfer")]
    const MAX_TRANSFER_UNITS: usize = 1 << 5;
}

impl From<Descriptor> for SerializedDescriptor {
    fn from(value: Descriptor) -> Self {
        Self {
            raw: [
                value.ctrl.into(),
                value.src as u32,
                value.dst as u32,
                value.link.into(),
            ],
        }
    }
}

/// Serialized DMA Descriptor
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(C)]
struct SerializedDescriptor {
    raw: [u32; 4],
}

/// CTRL register
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Ctrl {
    /// [31]
    dst_mode: AddrMode,
    /// [30]
    src_mode: AddrMode,
    /// [29:28]
    dst_inc: DSTINC,
    /// [27:26]
    size: SIZE,
    /// [25:24]
    src_inc: SRCINC,
    /// [23]
    ignore_s_req: bool,
    /// [22]
    dec_loop_cnt: bool,
    /// [21]
    req_mode: bool,
    /// Setting this bit will set the interrupt flag when the transfer is done, or linked in the case where the LINK
    /// bit is set, or synchronized in the case of a SYNC transfer.
    ///
    /// [20]
    done_if_s_en: bool,
    /// [19:16]
    block_size: BLOCKSIZE,
    /// [15]
    byte_swap: bool,
    /// The `XFERCNT` field specifies number of unit data (words, half-words, or bytes) to transfer, as determined
    /// by the SIZE field.
    /// **The value written should be one less than the desired transfer count**
    ///
    /// [14:4]
    xfer_cnt: u16,
    /// [3]
    struct_req: bool,
    /// [1:0]
    struct_type: STRUCTTYPE,
}

impl Ctrl {
    fn new() -> Self {
        Self::default()
    }

    fn with_size(mut self, unit: SIZE) -> Self {
        self.size = unit;
        self
    }

    fn with_struct_req(mut self) -> Self {
        self.struct_req = true;
        self
    }

    fn with_block_size(mut self, blocksize: BLOCKSIZE) -> Self {
        self.block_size = blocksize;
        self
    }

    /// Set the transfer count. Must be `<= Descriptor::MAX_TRANSFER_UNITS` (`0x800` units)
    ///
    /// Example:
    ///
    /// ```rust,no_run
    ///     Ctrl::new().with_xfer_cnt(0x800usize.try_into().unwrap())
    /// ```
    fn with_xfer_cnt(mut self, units: TransferCount) -> Ctrl {
        self.xfer_cnt = units.count - 1;
        self
    }
}

impl Default for Ctrl {
    fn default() -> Self {
        Self {
            struct_type: STRUCTTYPE::Transfer,
            struct_req: Default::default(),
            xfer_cnt: Default::default(),
            byte_swap: Default::default(),
            block_size: BLOCKSIZE::Unit1,
            done_if_s_en: Default::default(),
            req_mode: Default::default(),
            dec_loop_cnt: Default::default(),
            ignore_s_req: Default::default(),
            src_inc: SRCINC::One,
            size: SIZE::Byte,
            dst_inc: DSTINC::One,
            src_mode: Default::default(),
            dst_mode: Default::default(),
        }
    }
}

impl From<Ctrl> for u32 {
    fn from(value: Ctrl) -> Self {
        let mut ret = 0;
        ret |= (value.dst_mode as u32) << 31;
        ret |= (value.src_mode as u32) << 30;
        ret |= (value.dst_inc as u32) << 28;
        ret |= (value.size as u32) << 26;
        ret |= (value.src_inc as u32) << 24;
        ret |= (value.ignore_s_req as u32) << 23;
        ret |= (value.dec_loop_cnt as u32) << 22;
        ret |= (value.req_mode as u32) << 21;
        ret |= (value.done_if_s_en as u32) << 20;
        ret |= (value.block_size as u32) << 16;
        ret |= (value.byte_swap as u32) << 15;
        ret |= ((value.xfer_cnt as u32) & 0x3FFFFFFF) << 4;
        ret |= (value.struct_req as u32) << 3;
        ret |= value.struct_type as u32;
        ret
    }
}

/// Wrapper for the CTRL.XFERCNT which makes sure the value is at most `Descriptor::MAX_TRANSFER_UNITS` (`0x800`)
struct TransferCount {
    count: u16,
}

impl TryFrom<usize> for TransferCount {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value <= Descriptor::MAX_TRANSFER_UNITS {
            Ok(Self {
                count: value as u16,
            })
        } else {
            Err(())
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Link {
    /// Link Structure Address
    ///
    /// **WARNING:** the value of this field needs to be expressed in 32-bit words, not in bytes
    ///
    /// - For `Absolute` addressing, right-shift the byte addres by 2. E.g. if the descriptor is at `0x200056F4`,
    ///   then this field needs to contain `0x200056F4 >> 2`, which is `0x80015BD`
    ///
    /// - For `Relative` addressing, if you need to point to the next descriptor in memory, right-shift the size of
    ///   the descriptor by 2, so write `4` to point to the next descriptor, `8` to jump to the one after that, etc.
    ///
    /// [31:2]
    link_addr: usize,
    /// [1]
    link: bool,
    /// [0]
    link_mode: AddrMode,
}

impl Link {
    fn new() -> Self {
        Self::default()
    }

    fn with_absolute_addr(mut self, addr: usize) -> Link {
        self.link_addr = addr >> 2;
        self.link = true;
        self.link_mode = AddrMode::Absolute;
        self
    }

    fn with_relative_addr(mut self, count: isize) -> Link {
        self.link_addr = (count * size_of::<SerializedDescriptor>() as isize) as usize >> 2;
        self.link = true;
        self.link_mode = AddrMode::Relative;
        self
    }
}

impl From<Link> for u32 {
    fn from(value: Link) -> Self {
        let mut ret = 0;
        ret |= (value.link_addr as u32) << 2;
        ret |= (value.link as u32) << 1;
        ret |= value.link_mode as u32;
        ret
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum AddrMode {
    #[default]
    Absolute = 0,
    Relative = 1,
}

/// Register-level DMA functions
pub(crate) mod mmio {
    use crate::dma::{ChannelId, Descriptor};
    use crate::pac::Ldma;
    use crate::SingleCycleRMW;

    /// Get channel enabled
    pub(crate) fn ch_enabled(id: ChannelId) -> bool {
        dma().chen().read().chen().bits() & (1 << id as u8) != 0
    }

    /// Enable channel
    pub(crate) fn ch_enable_set(id: ChannelId) {
        dma().chen().sc_set(1 << id as u8);
    }

    /// Disable channel
    pub(crate) fn ch_enable_clear(id: ChannelId) {
        dma().chen().sc_clear(1 << id as u8);
    }

    pub(crate) fn ch_done(id: ChannelId) -> bool {
        dma().chdone().read().bits() & (1 << id as u8) != 0
    }

    pub(crate) fn ch_done_set(id: ChannelId) {
        dma().chdone().sc_set(1 << id as u8);
    }

    pub(crate) fn ch_done_clear(id: ChannelId) {
        dma().chdone().sc_clear(1 << id as u8);
    }

    pub(crate) fn ch_error(id: ChannelId) -> bool {
        dma().status().read().cherror().bits() == id as u8
    }

    pub(crate) fn ch_busy(id: ChannelId) -> bool {
        dma().chbusy().read().busy().bits() & (1 << id as u8) != 0
    }

    pub(crate) fn ien_set(id: ChannelId) {
        dma().ien().sc_set(1 << id as u8);
    }

    pub(crate) fn ien_clear(id: ChannelId) {
        dma().ien().sc_clear(1 << id as u8);
    }

    pub(crate) fn if_clear(id: ChannelId) {
        dma()
            .ifc()
            .write(|w| unsafe { w.done().bits(1 << id as u8) });
    }

    pub(crate) fn ch_start(id: ChannelId) {
        dma()
            .swreq()
            .write(|w| unsafe { w.swreq().bits(1 << id as u8) });
    }

    pub(crate) fn ch_link_load(id: ChannelId) {
        dma()
            .linkload()
            .write(|w| unsafe { w.linkload().bits(1 << id as u8) });
    }

    pub(crate) fn ch_req_mode_set(id: ChannelId, all: bool) {
        dma()
            .ch(id as usize)
            .ctrl()
            .modify(|_, w| w.reqmode().bit(all));
    }

    /// WARNING: number of words actually transfered will be `cnt + 1`
    pub(crate) fn ch_xfer_cnt_set(id: ChannelId, cnt: u16) {
        dma()
            .ch(id as usize)
            .ctrl()
            .write(|w| unsafe { w.xfercnt().bits(cnt) });
    }

    pub(crate) fn ch_src_set(id: ChannelId, addr: u32) {
        dma()
            .ch(id as usize)
            .src()
            .write(|w| unsafe { w.srcaddr().bits(addr) });
    }

    pub(crate) fn ch_dst_set(id: ChannelId, addr: u32) {
        dma()
            .ch(id as usize)
            .dst()
            .write(|w| unsafe { w.dstaddr().bits(addr) });
    }

    pub(crate) fn ch_write_descriptor(id: ChannelId, descr: &Descriptor) {
        dma()
            .ch(id as usize)
            .ctrl()
            .write(|w| unsafe { w.bits(descr.ctrl.into()) });
        dma()
            .ch(id as usize)
            .src()
            .write(|w| unsafe { w.bits(descr.src as u32) });
        dma()
            .ch(id as usize)
            .dst()
            .write(|w| unsafe { w.bits(descr.dst as u32) });
        dma()
            .ch(id as usize)
            .link()
            .write(|w| unsafe { w.bits(descr.link.into()) });
    }

    /// Get the DMA (pac) peripheral
    fn dma() -> Ldma {
        unsafe { crate::pac::Ldma::steal() }
    }
}
