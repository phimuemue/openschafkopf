fn has_trumpf_ober(ctx, player, trumpf, ober) {
    ctx.trumpf(player)==trumpf
        && ctx.ober(player)==ober
}

fn has_trumpf_ober_unter(ctx, player, trumpf, ober, unter) {
    has_trumpf_ober(ctx, player, trumpf, ober)
        && ctx.unter(player)==unter
}

fn any_has_trumpf_ober_unter(ctx, trumpf, ober, unter) {
    has_trumpf_ober_unter(ctx, 1, trumpf, ober, unter)
    || has_trumpf_ober_unter(ctx, 2, trumpf, ober, unter)
    || has_trumpf_ober_unter(ctx, 3, trumpf, ober, unter)
}

fn any_has_trumpf_ober(ctx, trumpf, ober) {
    has_trumpf_ober(ctx, 1, trumpf, ober)
    || has_trumpf_ober(ctx, 2, trumpf, ober)
    || has_trumpf_ober(ctx, 3, trumpf, ober)
}
