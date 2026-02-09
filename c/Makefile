# YAY Parser C Implementation - Makefile

CC = cc
CFLAGS = -Wall -Wextra -Werror -std=c11 -g -O2
LDFLAGS = -lm

# Source files
SRCS = yay.c
TEST_SRCS = test_yay.c

# Object files
OBJS = $(SRCS:.c=.o)
TEST_OBJS = $(TEST_SRCS:.c=.o)

# Targets
LIB = libyay.a
TEST = test_yay

.PHONY: all clean test fixtures

all: $(LIB) $(TEST)

# Generate fixtures from Go test files
fixtures: fixtures_gen.h

fixtures_gen.h: ../test/c/*.c ../test/yay/*.yay ../test/nay/*.nay gen_fixtures.py
	python3 gen_fixtures.py

# Build static library
$(LIB): $(OBJS)
	ar rcs $@ $^

# Build test runner
$(TEST): $(TEST_OBJS) $(LIB) fixtures_gen.h
	$(CC) $(CFLAGS) -o $@ $(TEST_OBJS) -L. -lyay $(LDFLAGS)

# Compile source files
%.o: %.c yay.h
	$(CC) $(CFLAGS) -c $< -o $@

# Test runner depends on generated fixtures
test_yay.o: test_yay.c yay.h fixtures_gen.h
	$(CC) $(CFLAGS) -c $< -o $@

# Run tests
test: $(TEST)
	./$(TEST)

# Run a specific test
test-%: $(TEST)
	./$(TEST) -t $*

# Clean build artifacts
clean:
	rm -f $(OBJS) $(TEST_OBJS) $(LIB) $(TEST)

# Clean everything including generated files
distclean: clean
	rm -f fixtures_gen.h

# Show help
help:
	@echo "YAY Parser C Implementation"
	@echo ""
	@echo "Targets:"
	@echo "  all        Build library and test runner (default)"
	@echo "  test       Run all tests"
	@echo "  test-NAME  Run a specific test (e.g., test-null_literal)"
	@echo "  fixtures   Regenerate fixtures from Go test files"
	@echo "  clean      Remove build artifacts"
	@echo "  distclean  Remove all generated files"
	@echo "  help       Show this help"
