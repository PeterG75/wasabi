(module
    (func $source (param i32))
    (export "taint_source" (func $source))
    (func $sink (param i32))
    (export "taint_sink" (func $sink))

    (global $globA i32 (i32.const 55))

    (func $f
        ;; mark globA as tainted
        get_global $globA
        call $source

        get_global $globA
        call $sink
    )

    (start $f)
)