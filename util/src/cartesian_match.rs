#[macro_export]
macro_rules! cartesian_match(
    (
        $macro_callback: ident,
        $(match ($e: expr) {
            $($x: pat => $y: tt,)*
        },)*
    ) => {
        cartesian_match!(@p0,
            $macro_callback,
            (),
            $(match ($e) {
                $($x => $y,)*
            },)*
        )
    };
    (@p0,
        $macro_callback: ident,
        $rest_packed: tt,
        match ($e: expr) {
            $($x: pat => $y: tt,)*
        },
        $(match ($e2: expr) {
            $($x2: pat => $y2: tt,)*
        },)*
    ) => {
        cartesian_match!(@p0,
            $macro_callback,
            (
                match ($e) {
                    $($x => $y,)*
                },
                $rest_packed,
            ),
            $(match ($e2) {
                $($x2 => $y2,)*
            },)*
        )
    };
    (@p0,
        $macro_callback: ident,
        $rest_packed: tt,
    ) => {
        cartesian_match!(@p1,
            $macro_callback,
            @matched{()},
            $rest_packed,
        )
    };
    (@p1,
        $macro_callback: ident,
        @matched{$matched_packed: tt},
        (
            match ($e: expr) {
                $($x: pat => $y: tt,)*
            },
            $rest_packed: tt,
        ),
    ) => {
        match $e {
            $($x => cartesian_match!(@p1,
                $macro_callback,
                @matched{ ($matched_packed, $y,) },
                $rest_packed,
            ),)*
        }
    };
    (@p1,
        $macro_callback: ident,
        @matched{$matched_packed: tt},
        (),
    ) => {
        cartesian_match!(@p2,
            $macro_callback,
            @unpacked(),
            $matched_packed,
        )
    };
    (@p2,
        $macro_callback: ident,
        @unpacked($($u: tt,)*),
        (
            $rest_packed: tt,
            $y: tt,
        ),
    ) => {
        cartesian_match!(@p2,
            $macro_callback,
            @unpacked($($u,)* $y,),
            $rest_packed,
        )
    };
    (@p2,
        $macro_callback: ident,
        @unpacked($($u: tt,)*),
        (),
    ) => {
        $macro_callback!($($u,)*)
    };
);


