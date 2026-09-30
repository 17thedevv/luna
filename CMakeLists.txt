cmake_minimum_required(VERSION 3.20)
project(luna-toolchain VERSION 1.0.0 LANGUAGES CXX C)

# ==========================================
# Root build system — Luna Toolchain
#
# Subprojects:
#   runtime/    → luna-runtime.lib / .a
# ==========================================

# ── Output layout ─────────────────────────────────────────────────────────────
# Build artifacts go to build/host/{platform}/{config}/
# This keeps source tree clean.

if(MSVC)
    set(LUNA_HOST_OS "windows-msvc")
elseif(WIN32 AND MINGW)
    set(LUNA_HOST_OS "windows-gnu")
elseif(UNIX AND APPLE)
    set(LUNA_HOST_OS "apple-darwin")
elseif(UNIX)
    set(LUNA_HOST_OS "unknown-linux-gnu")
else()
    set(LUNA_HOST_OS "unknown")
endif()

if(CMAKE_SYSTEM_PROCESSOR MATCHES "^(ARM64|arm64|aarch64)$")
    set(LUNA_HOST_ARCH "aarch64")
else()
    set(LUNA_HOST_ARCH "x86_64")
endif()

if(WIN32 AND LUNA_HOST_ARCH STREQUAL "aarch64")
    set(LUNA_HOST_TRIPLE "aarch64-pc-${LUNA_HOST_OS}")
elseif(WIN32)
    set(LUNA_HOST_TRIPLE "x86_64-pc-${LUNA_HOST_OS}")
elseif(APPLE)
    set(LUNA_HOST_TRIPLE "${LUNA_HOST_ARCH}-apple-darwin")
else()
    set(LUNA_HOST_TRIPLE "${LUNA_HOST_ARCH}-${LUNA_HOST_OS}")
endif()

set(CMAKE_RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/host/${LUNA_HOST_TRIPLE}/$<CONFIG>")
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/host/${LUNA_HOST_TRIPLE}/$<CONFIG>")
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/host/${LUNA_HOST_TRIPLE}/$<CONFIG>")

message(STATUS "Luna toolchain output: ${CMAKE_BINARY_DIR}/host/${LUNA_HOST_TRIPLE}/")

add_subdirectory(runtime)

enable_testing()
