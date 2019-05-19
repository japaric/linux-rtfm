set -euo pipefail

main() {
    local n=$1 bin=$2

    rm -f samples
    for i in $(seq 1 $n); do
        $bin >> samples
    done
}

main "${@}"
