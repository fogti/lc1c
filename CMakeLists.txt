cmake_minimum_required(VERSION 3.6)
project(lc1c)

set(INSTALL_BIN_DIR bin CACHE PATH "Installation directory for binaries")
set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Threads REQUIRED)
include_directories("${PROJECT_SOURCE_DIR}/src")

# LC1C compiler
add_executable(lc1c src/main.cxx src/utils.cxx)

target_link_libraries(lc1c Threads::Threads)
install(TARGETS lc1c DESTINATION "${INSTALL_BIN_DIR}")
