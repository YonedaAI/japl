CC = gcc
CFLAGS = -std=c11 -Wall -Wextra -Wpedantic -O2 -Iinclude -pthread -D_GNU_SOURCE
SRC = $(wildcard src/*.c)
OBJ = $(SRC:src/%.c=build/%.o)
LIB = libjapl_runtime.a
TEST_SRC = $(wildcard test/*.c)

.PHONY: all test clean

all: $(LIB)

build:
	@mkdir -p build

build/%.o: src/%.c | build
	$(CC) $(CFLAGS) -c $< -o $@

$(LIB): $(OBJ)
	ar rcs $@ $^

test: $(LIB)
	$(CC) $(CFLAGS) -o test_runner $(TEST_SRC) -L. -ljapl_runtime -lpthread
	./test_runner

clean:
	rm -rf build $(LIB) test_runner
