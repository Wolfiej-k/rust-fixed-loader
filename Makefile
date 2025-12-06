CC = clang-14
CFLAGS = -O2 -fPIC -Wall -Wextra
LDFLAGS = -shared -Wl,-z,now

DIR = programs
LOADER = ./target/release/fixed_loader

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

test-all: all
	@echo "Running all tests at once..."
	@$(LOADER) $(TEST_TARGETS)

$(DIR)/%.so: $(DIR)/%.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<

clean:
	rm -f $(DIR)/*.so
