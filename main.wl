std:displayln "STARTUP";
!:global on_input = {!(api, key) = @;
    $DEBUG "WL INPUT: " key api;
    api.set_prompt key $f;
};
