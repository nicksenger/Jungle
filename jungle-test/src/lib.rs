#[cfg(test)]
mod tests {
    use std::net::{Ipv6Addr, SocketAddr, UdpSocket};

    fn reserve_local_addr() -> SocketAddr {
        let socket = UdpSocket::bind((Ipv6Addr::LOCALHOST, 0))
            .expect("should bind temporary udp socket for test port reservation");
        socket
            .local_addr()
            .expect("temporary udp socket should expose local address")
    }

    mod adapt_helpers;
    mod aspect;
    mod conditional;
    mod connection;
    mod effect_attr;
    mod failure;
    mod generic_act;
    mod integration;
    mod migration;
    mod multi_worker;
    mod no_effect;
    mod optic;
    mod property;
    mod progression;
    mod replay;
    mod select_join;
    mod sleep;
    mod template_binding;
    mod transparent_metadata;
    mod traverse_replace;
    mod versioning;
    mod while_loop;
    mod zoo;
}
