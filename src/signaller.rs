pub trait Signaller {
    fn send(&self);
    fn recv_sdp_channel(&self) -> Recv<DescriptorChannel>;
}