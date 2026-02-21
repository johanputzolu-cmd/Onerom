#####################################################################
# PIO tests
#
# Uses epio PIO emulator to verify correct PIO behaviour
#####################################################################
set -e

run_test() {
    local hw_rev=$1
    local image=$2
    local base_config=$3
    local num_cs=$4
    local extra_flags=${5:-}

    for cs1 in 0 1; do
        if [ $num_cs -lt 2 ]; then
            local cmd="HW_REV=$hw_rev MCU=rp2350 EXTRA_C_FLAGS=\"$extra_flags\" ROM_CONFIGS=\"file=$image,$base_config,cs1=$cs1\" make test-pio"
            echo "$cmd"
            env HW_REV=$hw_rev MCU=rp2350 EXTRA_C_FLAGS="$extra_flags" \
                ROM_CONFIGS="file=$image,$base_config,cs1=$cs1" make test-pio > /dev/null || \
                { echo "FAILED: $cmd"; exit 1; }
            continue
        fi
        for cs2 in 0 1; do
            if [ $num_cs -lt 3 ]; then
                local cmd="HW_REV=$hw_rev MCU=rp2350 EXTRA_C_FLAGS=\"$extra_flags\" ROM_CONFIGS=\"file=$image,$base_config,cs1=$cs1,cs2=$cs2\" make test-pio"
                echo "$cmd"
                env HW_REV=$hw_rev MCU=rp2350 EXTRA_C_FLAGS="$extra_flags" \
                    ROM_CONFIGS="file=$image,$base_config,cs1=$cs1,cs2=$cs2" make test-pio > /dev/null || \
                    { echo "FAILED: $cmd"; exit 1; }
                continue
            fi
            for cs3 in 0 1; do
                local cmd="HW_REV=$hw_rev MCU=rp2350 EXTRA_C_FLAGS=\"$extra_flags\" ROM_CONFIGS=\"file=$image,$base_config,cs1=$cs1,cs2=$cs2,cs3=$cs3\" make test-pio"
                echo "$cmd"
                env HW_REV=$hw_rev MCU=rp2350 EXTRA_C_FLAGS="$extra_flags" \
                    ROM_CONFIGS="file=$image,$base_config,cs1=$cs1,cs2=$cs2,cs3=$cs3" make test-pio > /dev/null || \
                    { echo "FAILED: $cmd"; exit 1; }
            done
        done
    done
}

run_no_cs() {
    local hw_rev=$1
    local image=$2
    local base_config=$3
    local extra_flags=${4:-}

    local cmd="HW_REV=$hw_rev MCU=rp2350 EXTRA_C_FLAGS=\"$extra_flags\" ROM_CONFIGS=\"file=$image,$base_config\" make test-pio"
    echo "$cmd"
    env HW_REV=$hw_rev MCU=rp2350 EXTRA_C_FLAGS="$extra_flags" \
        ROM_CONFIGS="file=$image,$base_config" make test-pio > /dev/null || \
        { echo "FAILED: $cmd"; exit 1; }
}

test_24pin() {
    local hw_rev=${1:-fire-24-e}
    local extra_flags=${2:-}

    run_test   $hw_rev images/test/rand_8192.rom trunc,type=2316 3 "$extra_flags"
    run_test   $hw_rev images/test/rand_8192.rom trunc,type=2332 2 "$extra_flags"
    run_test   $hw_rev images/test/rand_8192.rom type=2364       1 "$extra_flags"
    run_no_cs  $hw_rev images/test/rand_8192.rom trunc,type=2716   "$extra_flags"
    run_no_cs  $hw_rev images/test/rand_8192.rom trunc,type=2732   "$extra_flags"
}

test_28pin() {
    local hw_rev=${1:-fire-28-a}
    local extra_flags=${2:-}

    run_test   $hw_rev images/test/rand_65536.rom trunc,type=23128 3 "$extra_flags"
    run_test   $hw_rev images/test/rand_65536.rom trunc,type=23256 2 "$extra_flags"
    run_test   $hw_rev images/test/rand_65536.rom type=23512       2 "$extra_flags"
    run_test   $hw_rev images/test/rand_128KB.rom type=231024      1 "$extra_flags"
    run_no_cs  $hw_rev images/test/rand_65536.rom trunc,type=2764    "$extra_flags"
    run_no_cs  $hw_rev images/test/rand_65536.rom trunc,type=27128   "$extra_flags"
    run_no_cs  $hw_rev images/test/rand_65536.rom trunc,type=27256   "$extra_flags"
    run_no_cs  $hw_rev images/test/rand_65536.rom type=27512         "$extra_flags"
}

test_40pin() {
    local hw_rev=${1:-fire-40-a}
    local extra_flags=${2:-}

    run_no_cs  $hw_rev images/test/rand_512KB.rom type=27C400 "$extra_flags"
}

test_24pin fire-24-a -DRP_PIO
test_24pin fire-24-b -DRP_PIO
test_24pin fire-24-c
test_24pin fire-24-d
test_24pin fire-24-e

test_28pin fire-28-a

# PIO tester doesn't support 40 pin ROMs yet
#test_40pin fire-40-a