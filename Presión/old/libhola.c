#include <stdio.h>
#include <string.h>
#include <pthread.h>
#include <unistd.h>

static char buffer[128];
static pthread_t thread;
static int running = 0;

void* loop(void* arg) {
    const char* msg = "hola";
    while (running) {
        snprintf(buffer, sizeof(buffer), "%s", msg);
        usleep(50000); // 20 Hz -> 50 ms
    }
    return NULL;
}

void start_hola() {
    if (!running) {
        running = 1;
        pthread_create(&thread, NULL, loop, NULL);
    }
}

void stop_hola() {
    if (running) {
        running = 0;
        pthread_join(thread, NULL);
    }
}

void get_data(char* out, int size) {
    snprintf(out, size, "%s", buffer);
}
