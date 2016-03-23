all: src/main.c
	gcc -Wall -o armgb src/*.c -DDEBUG=1

clean:
	 rm *.o
