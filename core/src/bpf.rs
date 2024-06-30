
extern "C" {
    fn socket_attach_bpf_filter(socket: std::os::raw::c_int) -> bool;
}

pub fn socket_attach_filter(socket: &impl std::os::fd::AsRawFd) -> bool {
    let fd = socket.as_raw_fd();
    unsafe {
        socket_attach_bpf_filter(fd)
    }
}
