// =============================================================================
// runtime/src/platform/posix/net.c
//
// Luna Runtime — Networking ABI (POSIX Platform Implementation)
// =============================================================================

#include "luna/runtime/net.h"
#include <sys/socket.h>
#include <sys/time.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <arpa/inet.h>
#include <netdb.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <string.h>
#include <stdlib.h>

int __luna_net_init(void) {
    return 0; // POSIX doesn't require socket subsystem init
}

LunaSocketHandle __luna_tcp_listen(uint32_t ipv4_host, uint16_t port, int backlog) {
    int s = socket(AF_INET, SOCK_STREAM, 0);
    if (s < 0) return -1;

    int opt = 1;
    setsockopt(s, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(ipv4_host);
    addr.sin_port = htons(port);

    if (bind(s, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        close(s);
        return -1;
    }

    if (listen(s, backlog > 0 ? backlog : SOMAXCONN) < 0) {
        close(s);
        return -1;
    }

    return (LunaSocketHandle)s;
}

LunaSocketHandle __luna_tcp_accept(LunaSocketHandle listener, uint32_t* out_ip, uint16_t* out_port) {
    if (listener < 0) return -1;
    struct sockaddr_in client_addr;
    socklen_t addr_len = sizeof(client_addr);
    memset(&client_addr, 0, sizeof(client_addr));

    int client = accept((int)listener, (struct sockaddr*)&client_addr, &addr_len);
    if (client < 0) return -1;

    if (out_ip) {
        *out_ip = ntohl(client_addr.sin_addr.s_addr);
    }
    if (out_port) {
        *out_port = ntohs(client_addr.sin_port);
    }

    return (LunaSocketHandle)client;
}

LunaSocketHandle __luna_tcp_connect(uint32_t ipv4_host, uint16_t port) {
    int s = socket(AF_INET, SOCK_STREAM, 0);
    if (s < 0) return -1;

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(ipv4_host);
    addr.sin_port = htons(port);

    if (connect(s, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        close(s);
        return -1;
    }

    return (LunaSocketHandle)s;
}

int64_t __luna_tcp_read(LunaSocketHandle sock, void* buf, uint64_t len) {
    if (sock < 0 || !buf) return -1;
    ssize_t res = recv((int)sock, buf, (size_t)len, 0);
    return (int64_t)res;
}

int64_t __luna_tcp_write(LunaSocketHandle sock, const void* buf, uint64_t len) {
    if (sock < 0 || !buf) return -1;
    ssize_t res = send((int)sock, buf, (size_t)len, 0);
    return (int64_t)res;
}

int __luna_tcp_shutdown(LunaSocketHandle sock, int how) {
    if (sock < 0) return -1;
    int p_how = SHUT_RDWR;
    if (how == 0) p_how = SHUT_RD;
    else if (how == 1) p_how = SHUT_WR;
    return shutdown((int)sock, p_how);
}

LunaSocketHandle __luna_udp_bind(uint32_t ipv4_host, uint16_t port) {
    int s = socket(AF_INET, SOCK_DGRAM, 0);
    if (s < 0) return -1;

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(ipv4_host);
    addr.sin_port = htons(port);

    if (bind(s, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        close(s);
        return -1;
    }

    return (LunaSocketHandle)s;
}

int64_t __luna_udp_send_to(LunaSocketHandle sock, const void* buf, uint64_t len, uint32_t ipv4_host, uint16_t port) {
    if (sock < 0 || !buf) return -1;
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(ipv4_host);
    addr.sin_port = htons(port);

    ssize_t res = sendto((int)sock, buf, (size_t)len, 0, (struct sockaddr*)&addr, sizeof(addr));
    return (int64_t)res;
}

int64_t __luna_udp_recv_from(LunaSocketHandle sock, void* buf, uint64_t len, uint32_t* out_ip, uint16_t* out_port) {
    if (sock < 0 || !buf) return -1;
    struct sockaddr_in addr;
    socklen_t addr_len = sizeof(addr);
    memset(&addr, 0, sizeof(addr));

    ssize_t res = recvfrom((int)sock, buf, (size_t)len, 0, (struct sockaddr*)&addr, &addr_len);
    if (res < 0) return -1;

    if (out_ip) *out_ip = ntohl(addr.sin_addr.s_addr);
    if (out_port) *out_port = ntohs(addr.sin_port);
    return (int64_t)res;
}

int __luna_socket_close(LunaSocketHandle sock) {
    if (sock < 0) return 0;
    return close((int)sock);
}

int __luna_socket_local_addr(LunaSocketHandle sock, uint32_t* out_ip, uint16_t* out_port) {
    if (sock < 0) return -1;
    struct sockaddr_in addr;
    socklen_t len = sizeof(addr);
    if (getsockname((int)sock, (struct sockaddr*)&addr, &len) < 0) {
        return -1;
    }
    if (out_ip) *out_ip = ntohl(addr.sin_addr.s_addr);
    if (out_port) *out_port = ntohs(addr.sin_port);
    return 0;
}

int __luna_socket_peer_addr(LunaSocketHandle sock, uint32_t* out_ip, uint16_t* out_port) {
    if (sock < 0) return -1;
    struct sockaddr_in addr;
    socklen_t len = sizeof(addr);
    if (getpeername((int)sock, (struct sockaddr*)&addr, &len) < 0) {
        return -1;
    }
    if (out_ip) *out_ip = ntohl(addr.sin_addr.s_addr);
    if (out_port) *out_port = ntohs(addr.sin_port);
    return 0;
}

int __luna_socket_set_timeout(LunaSocketHandle sock, int is_read, uint64_t timeout_ms) {
    if (sock < 0) return -1;
    struct timeval tv;
    tv.tv_sec = timeout_ms / 1000;
    tv.tv_usec = (timeout_ms % 1000) * 1000;
    int opt = is_read ? SO_RCVTIMEO : SO_SNDTIMEO;
    return setsockopt((int)sock, SOL_SOCKET, opt, &tv, sizeof(tv));
}

int __luna_socket_set_nodelay(LunaSocketHandle sock, int nodelay) {
    if (sock < 0) return -1;
    int opt = nodelay ? 1 : 0;
    return setsockopt((int)sock, IPPROTO_TCP, TCP_NODELAY, &opt, sizeof(opt));
}

int __luna_socket_set_nonblocking(LunaSocketHandle sock, int nonblocking) {
    if (sock < 0) return -1;
    int flags = fcntl((int)sock, F_GETFL, 0);
    if (flags < 0) return -1;
    if (nonblocking) {
        flags |= O_NONBLOCK;
    } else {
        flags &= ~O_NONBLOCK;
    }
    return fcntl((int)sock, F_SETFL, flags);
}

int __luna_socket_set_broadcast(LunaSocketHandle sock, int broadcast) {
    if (sock < 0) return -1;
    int opt = broadcast ? 1 : 0;
    return setsockopt((int)sock, SOL_SOCKET, SO_BROADCAST, &opt, sizeof(opt));
}

int __luna_socket_last_error(void) {
    return errno;
}

int32_t __luna_net_resolve(
    const uint8_t*  host_ptr,
    size_t          host_len,
    uint32_t*       out_ips,
    size_t          max_ips,
    size_t*         out_count
) {
    if (!host_ptr || host_len == 0 || !out_ips || max_ips == 0 || !out_count) {
        return -1;
    }
    *out_count = 0;

    char stack_buf[256];
    char* hostname = stack_buf;
    if (host_len + 1 > sizeof(stack_buf)) {
        hostname = (char*)malloc(host_len + 1);
        if (!hostname) return -1;
    }
    memcpy(hostname, host_ptr, host_len);
    hostname[host_len] = '\0';

    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET; // IPv4
    hints.ai_socktype = SOCK_STREAM;

    struct addrinfo* res = NULL;
    int err = getaddrinfo(hostname, NULL, &hints, &res);
    if (hostname != stack_buf) free(hostname);
    if (err != 0 || !res) {
        return -1;
    }

    size_t count = 0;
    struct addrinfo* cur = res;
    while (cur && count < max_ips) {
        if (cur->ai_family == AF_INET && cur->ai_addr) {
            struct sockaddr_in* sin = (struct sockaddr_in*)cur->ai_addr;
            out_ips[count++] = ntohl(sin->sin_addr.s_addr);
        }
        cur = cur->ai_next;
    }

    freeaddrinfo(res);
    *out_count = count;
    return count > 0 ? 0 : -1;
}
