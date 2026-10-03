// =============================================================================
// runtime/src/platform/windows/net.c
//
// Luna Runtime — Networking ABI (Windows WinSock2 Platform Implementation)
// =============================================================================

#include "luna/runtime/net.h"
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <winsock2.h>
#include <ws2tcpip.h>
#include <stdlib.h>
#include <string.h>

static int net_initialized = 0;

int __luna_net_init(void) {
    if (!net_initialized) {
        WSADATA wsa;
        if (WSAStartup(MAKEWORD(2, 2), &wsa) == 0) {
            net_initialized = 1;
        }
    }
    return net_initialized ? 0 : -1;
}

LunaSocketHandle __luna_tcp_listen(uint32_t ipv4_host, uint16_t port, int backlog) {
    if (__luna_net_init() != 0) return -1;

    SOCKET s = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (s == INVALID_SOCKET) return -1;

    int opt = 1;
    setsockopt(s, SOL_SOCKET, SO_REUSEADDR, (const char*)&opt, sizeof(opt));

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(ipv4_host);
    addr.sin_port = htons(port);

    if (bind(s, (struct sockaddr*)&addr, sizeof(addr)) == SOCKET_ERROR) {
        closesocket(s);
        return -1;
    }

    if (listen(s, backlog > 0 ? backlog : SOMAXCONN) == SOCKET_ERROR) {
        closesocket(s);
        return -1;
    }

    return (LunaSocketHandle)s;
}

LunaSocketHandle __luna_tcp_accept(LunaSocketHandle listener, uint32_t* out_ip, uint16_t* out_port) {
    if (listener < 0) return -1;
    struct sockaddr_in client_addr;
    int addr_len = sizeof(client_addr);
    memset(&client_addr, 0, sizeof(client_addr));

    SOCKET client = accept((SOCKET)listener, (struct sockaddr*)&client_addr, &addr_len);
    if (client == INVALID_SOCKET) return -1;

    if (out_ip) {
        *out_ip = ntohl(client_addr.sin_addr.s_addr);
    }
    if (out_port) {
        *out_port = ntohs(client_addr.sin_port);
    }

    return (LunaSocketHandle)client;
}

LunaSocketHandle __luna_tcp_connect(uint32_t ipv4_host, uint16_t port) {
    if (__luna_net_init() != 0) return -1;

    SOCKET s = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (s == INVALID_SOCKET) return -1;

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(ipv4_host);
    addr.sin_port = htons(port);

    if (connect(s, (struct sockaddr*)&addr, sizeof(addr)) == SOCKET_ERROR) {
        closesocket(s);
        return -1;
    }

    return (LunaSocketHandle)s;
}

int64_t __luna_tcp_read(LunaSocketHandle sock, void* buf, uint64_t len) {
    if (sock < 0 || !buf) return -1;
    int to_read = len > 0x7FFFFFFF ? 0x7FFFFFFF : (int)len;
    int res = recv((SOCKET)sock, (char*)buf, to_read, 0);
    if (res == SOCKET_ERROR) {
        return -1;
    }
    return (int64_t)res;
}

int64_t __luna_tcp_write(LunaSocketHandle sock, const void* buf, uint64_t len) {
    if (sock < 0 || !buf) return -1;
    int to_write = len > 0x7FFFFFFF ? 0x7FFFFFFF : (int)len;
    int res = send((SOCKET)sock, (const char*)buf, to_write, 0);
    if (res == SOCKET_ERROR) {
        return -1;
    }
    return (int64_t)res;
}

int __luna_tcp_shutdown(LunaSocketHandle sock, int how) {
    if (sock < 0) return -1;
    int w_how = SD_BOTH;
    if (how == 0) w_how = SD_RECEIVE;
    else if (how == 1) w_how = SD_SEND;
    return shutdown((SOCKET)sock, w_how) == 0 ? 0 : -1;
}

LunaSocketHandle __luna_udp_bind(uint32_t ipv4_host, uint16_t port) {
    if (__luna_net_init() != 0) return -1;

    SOCKET s = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    if (s == INVALID_SOCKET) return -1;

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(ipv4_host);
    addr.sin_port = htons(port);

    if (bind(s, (struct sockaddr*)&addr, sizeof(addr)) == SOCKET_ERROR) {
        closesocket(s);
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

    int to_write = len > 0x7FFFFFFF ? 0x7FFFFFFF : (int)len;
    int res = sendto((SOCKET)sock, (const char*)buf, to_write, 0, (struct sockaddr*)&addr, sizeof(addr));
    if (res == SOCKET_ERROR) return -1;
    return (int64_t)res;
}

int64_t __luna_udp_recv_from(LunaSocketHandle sock, void* buf, uint64_t len, uint32_t* out_ip, uint16_t* out_port) {
    if (sock < 0 || !buf) return -1;
    struct sockaddr_in addr;
    int addr_len = sizeof(addr);
    memset(&addr, 0, sizeof(addr));

    int to_read = len > 0x7FFFFFFF ? 0x7FFFFFFF : (int)len;
    int res = recvfrom((SOCKET)sock, (char*)buf, to_read, 0, (struct sockaddr*)&addr, &addr_len);
    if (res == SOCKET_ERROR) return -1;

    if (out_ip) *out_ip = ntohl(addr.sin_addr.s_addr);
    if (out_port) *out_port = ntohs(addr.sin_port);
    return (int64_t)res;
}

int __luna_socket_close(LunaSocketHandle sock) {
    if (sock < 0) return 0;
    return closesocket((SOCKET)sock) == 0 ? 0 : -1;
}

int __luna_socket_local_addr(LunaSocketHandle sock, uint32_t* out_ip, uint16_t* out_port) {
    if (sock < 0) return -1;
    struct sockaddr_in addr;
    int len = sizeof(addr);
    if (getsockname((SOCKET)sock, (struct sockaddr*)&addr, &len) == SOCKET_ERROR) {
        return -1;
    }
    if (out_ip) *out_ip = ntohl(addr.sin_addr.s_addr);
    if (out_port) *out_port = ntohs(addr.sin_port);
    return 0;
}

int __luna_socket_peer_addr(LunaSocketHandle sock, uint32_t* out_ip, uint16_t* out_port) {
    if (sock < 0) return -1;
    struct sockaddr_in addr;
    int len = sizeof(addr);
    if (getpeername((SOCKET)sock, (struct sockaddr*)&addr, &len) == SOCKET_ERROR) {
        return -1;
    }
    if (out_ip) *out_ip = ntohl(addr.sin_addr.s_addr);
    if (out_port) *out_port = ntohs(addr.sin_port);
    return 0;
}

int __luna_socket_set_timeout(LunaSocketHandle sock, int is_read, uint64_t timeout_ms) {
    if (sock < 0) return -1;
    DWORD ms = (DWORD)timeout_ms;
    int opt = is_read ? SO_RCVTIMEO : SO_SNDTIMEO;
    return setsockopt((SOCKET)sock, SOL_SOCKET, opt, (const char*)&ms, sizeof(ms)) == 0 ? 0 : -1;
}

int __luna_socket_set_nodelay(LunaSocketHandle sock, int nodelay) {
    if (sock < 0) return -1;
    int opt = nodelay ? 1 : 0;
    return setsockopt((SOCKET)sock, IPPROTO_TCP, TCP_NODELAY, (const char*)&opt, sizeof(opt)) == 0 ? 0 : -1;
}

int __luna_socket_set_nonblocking(LunaSocketHandle sock, int nonblocking) {
    if (sock < 0) return -1;
    u_long mode = nonblocking ? 1 : 0;
    return ioctlsocket((SOCKET)sock, FIONBIO, &mode) == 0 ? 0 : -1;
}

int __luna_socket_set_broadcast(LunaSocketHandle sock, int broadcast) {
    if (sock < 0) return -1;
    int opt = broadcast ? 1 : 0;
    return setsockopt((SOCKET)sock, SOL_SOCKET, SO_BROADCAST, (const char*)&opt, sizeof(opt)) == 0 ? 0 : -1;
}

int __luna_socket_last_error(void) {
    return WSAGetLastError();
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

    if (__luna_net_init() != 0) {
        if (hostname != stack_buf) free(hostname);
        return -1;
    }

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
