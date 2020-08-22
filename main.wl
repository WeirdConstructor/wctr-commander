std:displayln "STARTUP";
!:global on_input = {!(key) = @;
    $DEBUG "WL INPUT: " key;
};
