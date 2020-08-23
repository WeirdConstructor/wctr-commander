std:displayln "STARTUP";
!mode = :normal;

!:global on_input = {!(api, key) = @;
    $DEBUG "WL INPUT:" key;
    match key
        "I" => {
            ? mode == :normal {
                api.set_prompt "> " $t;
                .mode = :wait_i;
            };
        }
        "Escape" => {
            api.set_prompt "[NORMAL]" $f;
            .mode = :normal;
        };
};

!:global on_text = {!(api, txt) = @;
    std:displayln "XXXXXXXXXXXXXXXXXXXXX" mode ";" txt;
    ? mode == :wait_i &and txt == "i" {
        .mode = :insert;
    } {
        ? mode == :insert {
            $DEBUG "TXT INPUT:" txt;
            api.text_insert txt;
        };
    };
};
