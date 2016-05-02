# OBJS specifies which files to compile as part of the project
OBJS = src/*.c

# CC specifies which compiler we're using
CC = gcc

# INCLUDE_PATHS specifies the additional include paths we'll need
INCLUDE_PATHS = -I/usr/local/include -I/opt/X11/include

# LIBRARY_PATHS specifies the additional library paths we'll need
LIBRARY_PATHS = -L/usr/local/lib -I/opt/X11/lib

# COMPILER_FLAGS specifies the additional compilation options we're using
# -w suppresses all warnings
COMPILER_FLAGS = -O3 -w -DDEBUG=1

# LINKER_FLAGS specifies the libraries we're linking against
# Cocoa, IOKit, and CoreVideo are needed for static GLFW3.
LINKER_FLAGS = `sdl2-config --cflags --libs` -lpthread

# OBJ_NAME specifies the name of our exectuable
OBJ_NAME = mcugb

#This is the target that compiles our executable
all : $(OBJS)
	$(CC) $(OBJS) $(INCLUDE_PATHS) $(LIBRARY_PATHS) $(COMPILER_FLAGS) $(LINKER_FLAGS) -o $(OBJ_NAME)

#all: src/main.c
#gcc -Wall -o mcugb src/*.c -DDEBUG=1

clean:
	 rm *.o
