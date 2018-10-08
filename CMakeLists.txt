cmake_minimum_required(VERSION 3.6)
project(lc1c)

set(INSTALL_BIN_DIR bin CACHE PATH "Installation directory for binaries")
set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_FLAGS "${CXXFLAGS} ${CMAKE_CXX_FLAGS} -fno-rtti")

include_directories("${PROJECT_SOURCE_DIR}/src")
add_executable(lc1c src/main.cxx src/optimize.cxx src/utils.cxx)
install(TARGETS lc1c DESTINATION "${INSTALL_BIN_DIR}")
