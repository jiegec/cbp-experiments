cmake_minimum_required(VERSION 3.22)
project(dynamorio)

# for speedup
set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -march=native -O3")

find_package(zstd REQUIRED)

add_library(brtrace SHARED brtrace.c utils.c)
target_link_libraries(brtrace zstd)

find_package(DynamoRIO)
if (NOT DynamoRIO_FOUND)
  message(FATAL_ERROR "DynamoRIO package required to build")
endif(NOT DynamoRIO_FOUND)

configure_DynamoRIO_client(brtrace)
use_DynamoRIO_extension(brtrace drmgr)
use_DynamoRIO_extension(brtrace drx)
