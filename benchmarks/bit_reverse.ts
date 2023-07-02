function bit_reverse8(x: bigint): bigint {
    var res = 0n;
    for (var i = 0n; i < 8n; i = i + 1n) {
        var last_bit = x - x / 2n * 2n;
        res = res * 2n + last_bit;
        x = x / 2n;
    }
    return res;
}

for (let i = 0n; i < 100000n; i = i + 1n) {
    var b = bit_reverse8(i);
}
