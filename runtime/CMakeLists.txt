cmake_minimum_required(VERSION 3.20)

project(luna-runtime VERSION 1.0.0 LANGUAGES C CXX)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

if(MSVC)
    set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
endif()

# Runtime ABI include directory (shared with compiler)
set(LUNA_RUNTIME_INCLUDE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/include" CACHE PATH "Luna runtime include directory")

include_directories(${LUNA_RUNTIME_INCLUDE_DIR})

# ── Platform detection ────────────────────────────────────────────────────────
if(WIN32)
    set(LUNA_PLATFORM "windows")
    set(LUNA_PLATFORM_SOURCE "windows")
elseif(UNIX AND APPLE)
    set(LUNA_PLATFORM "macos")
    set(LUNA_PLATFORM_SOURCE "posix")
elseif(UNIX)
    set(LUNA_PLATFORM "linux")
    set(LUNA_PLATFORM_SOURCE "posix")
else()
    set(LUNA_PLATFORM "freestanding")
    set(LUNA_PLATFORM_SOURCE "freestanding")
endif()

message(STATUS "Luna Runtime: building for platform '${LUNA_PLATFORM}'")

# ── Source files ─────────────────────────────────────────────────────────────
set(RUNTIME_CORE_SRC
    src/memory/memory.c
    src/panic/panic.c
    src/io/io.c
    src/startup/startup.c
)

set(RUNTIME_PLATFORM_SRC
    src/platform/${LUNA_PLATFORM_SOURCE}/threading.c
    src/platform/${LUNA_PLATFORM_SOURCE}/sync.c
    src/platform/${LUNA_PLATFORM_SOURCE}/time.c
    src/platform/${LUNA_PLATFORM_SOURCE}/net.c
    src/platform/${LUNA_PLATFORM_SOURCE}/random.c
)

# ── Static library target ─────────────────────────────────────────────────────
add_library(luna-runtime STATIC
    ${RUNTIME_CORE_SRC}
    ${RUNTIME_PLATFORM_SRC}
)

target_include_directories(luna-runtime PUBLIC
    ${LUNA_RUNTIME_INCLUDE_DIR}
)

# Link pthreads on POSIX and ws2_32/bcrypt on Windows
if(UNIX)
    target_compile_definitions(luna-runtime PUBLIC _POSIX_C_SOURCE=200809L _DEFAULT_SOURCE)
    if(APPLE)
        target_compile_definitions(luna-runtime PUBLIC _DARWIN_C_SOURCE)
    endif()
    target_link_libraries(luna-runtime PRIVATE pthread)
endif()
if(WIN32)
    target_link_libraries(luna-runtime PRIVATE ws2_32 bcrypt)
endif()

# ── Install ───────────────────────────────────────────────────────────────────
install(TARGETS luna-runtime
    ARCHIVE DESTINATION runtime/hosted/${LUNA_PLATFORM}
)

install(DIRECTORY include/
    DESTINATION include
)

# ── Conformance Tests ─────────────────────────────────────────────────────────
enable_testing()
add_executable(abi_tests tests/abi_tests.c)
target_link_libraries(abi_tests PRIVATE luna-runtime)
target_include_directories(abi_tests PRIVATE ${LUNA_RUNTIME_INCLUDE_DIR})
add_test(NAME RuntimeAbiTests COMMAND abi_tests)
