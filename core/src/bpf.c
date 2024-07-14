#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <unistd.h>
#include <linux/bpf.h>
#include <linux/filter.h>
#include <sys/socket.h>
#include <arpa/inet.h>

#define SPEEDNET_MAGIC 0xFEEDCAFE

extern bool socket_attach_bpf_filter(int socket);

/**
 * Attach Speednet BPF Filter to the specified socket
 *
 * The BPF filter can be validated with:
 * printf "\xFE\xED\xCA\xFE_SpeednetMsg" | nc -u 127.0.0.1 4000 -w0
 */
bool socket_attach_bpf_filter(int socket) {
    /* check than first packet word (4 bytes) starts with SPEEDNET_MAGIC */
    struct sock_filter sock_filter[] = {
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, 8),                      /* LOAD First WORD after UDP Header */
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, htonl(SPEEDNET_MAGIC), 0, 1),  /* SPEEDNET_MAGIC ? */
        BPF_STMT(BPF_RET | BPF_K, 0x0fffffff),                      /* pass */
        BPF_STMT(BPF_RET | BPF_K, 0),                               /* reject */
    };
    struct sock_fprog sock_fprog = {
        .len = sizeof(sock_filter) / sizeof(*sock_filter),
        .filter = sock_filter,
    };
    if(setsockopt(socket, SOL_SOCKET, SO_ATTACH_FILTER, &sock_fprog, sizeof(sock_fprog)) < 0) {
        fprintf(stderr, "Failed to attach BPF Fitler to socket %d: %m\n", socket);
        return false;
    }

    return true;
}

