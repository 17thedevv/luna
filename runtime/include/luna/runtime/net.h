// =============================================================================
// luna/runtime/net.h
//
// Luna Runtime Networking ABI — Cross-platform TCP/UDP OS socket bindings.
// =============================================================================

#pragma once

#include "abi.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef int64_t LunaSocketHandle; // -1 = invalid/error

// Network subsystem initialization (called automatically upon socket creation)
int __luna_net_init(void);

// TCP Listener
LunaSocketHandle __luna_tcp_listen(uint32_t ipv4_host, uint16_t port, int backlog);
LunaSocketHandle __luna_tcp_accept(LunaSocketHandle listener, uint32_t* out_ip, uint16_t* out_port);

// TCP Stream
LunaSocketHandle __luna_tcp_connect(uint32_t ipv4_host, uint16_t port);
int64_t          __luna_tcp_read(LunaSocketHandle sock, void* buf, uint64_t len);
int64_t          __luna_tcp_write(LunaSocketHandle sock, const void* buf, uint64_t len);
int              __luna_tcp_shutdown(LunaSocketHandle sock, int how); // 0=read, 1=write, 2=both

// UDP Socket
LunaSocketHandle __luna_udp_bind(uint32_t ipv4_host, uint16_t port);
int64_t          __luna_udp_send_to(LunaSocketHandle sock, const void* buf, uint64_t len, uint32_t ipv4_host, uint16_t port);
int64_t          __luna_udp_recv_from(LunaSocketHandle sock, void* buf, uint64_t len, uint32_t* out_ip, uint16_t* out_port);

// Socket Lifecycle & Options
int              __luna_socket_close(LunaSocketHandle sock);
int              __luna_socket_local_addr(LunaSocketHandle sock, uint32_t* out_ip, uint16_t* out_port);
int              __luna_socket_peer_addr(LunaSocketHandle sock, uint32_t* out_ip, uint16_t* out_port);
int              __luna_socket_set_timeout(LunaSocketHandle sock, int is_read, uint64_t timeout_ms);
int              __luna_socket_set_nodelay(LunaSocketHandle sock, int nodelay);
int              __luna_socket_set_nonblocking(LunaSocketHandle sock, int nonblocking);
int              __luna_socket_set_broadcast(LunaSocketHandle sock, int broadcast);

// Last socket error code
int              __luna_socket_last_error(void);

// DNS Host Resolution
int32_t          __luna_net_resolve(
    const uint8_t*  host_ptr,
    size_t          host_len,
    uint32_t*       out_ips,
    size_t          max_ips,
    size_t*         out_count
);

#ifdef __cplusplus
} // extern "C"
#endif
