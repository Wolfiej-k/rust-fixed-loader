CC = gcc
CFLAGS = -fPIC -Wall -Wextra
LDFLAGS = -shared -Wl,-z,now

DIR = programs
SRCS = $(wildcard $(DIR)/*.c)
TARGETS = $(SRCS:.c=.so)

all: $(TARGETS)

$(DIR)/%.so: $(DIR)/%.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<

clean:
	rm -f $(DIR)/*.so
