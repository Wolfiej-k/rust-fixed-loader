CC = gcc
CFLAGS = -fPIC -Wall -Wextra
LDFLAGS = -shared -Wl,-z,now

DIR = programs
LOADER = ./target/release/rust_loader

SRCS = $(wildcard $(DIR)/*.c)
TARGETS = $(SRCS:.c=.so)

TESTS = $(wildcard $(DIR)/test_*.c)
TEST_TARGETS = $(TESTS:.c=.so)

all: $(TARGETS)

test: all
	@echo "Running all tests..."
	@for test in $(TEST_TARGETS); do \
		echo ""; \
		echo "=== $$test ==="; \
		$(LOADER) $$test || exit 1; \
	done
	@echo ""
	@echo "All tests passed!"

$(DIR)/%.so: $(DIR)/%.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<

clean:
	rm -f $(DIR)/*.so
