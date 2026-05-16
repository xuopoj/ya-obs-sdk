# bash completion for ya-obs
# Install: ya-obs completion bash > ~/.local/share/bash-completion/completions/ya-obs
# (or source it from your .bashrc)

_ya_obs() {
    local cur prev words cword
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    COMPREPLY=()

    # --- dynamic: obs:// path completion ---
    if [[ "$cur" == obs://* || "$cur" == obs:/ || "$cur" == obs: ]]; then
        # Only attempt dynamic completion when the user has typed at least "obs://"
        if [[ "$cur" == obs://* ]]; then
            local IFS=$'\n'
            local cands
            cands=$(command ya-obs --quiet __complete-obs -- "$cur" 2>/dev/null)
            if [[ -n "$cands" ]]; then
                while IFS= read -r line; do
                    [[ "$line" == "$cur"* ]] && COMPREPLY+=("$line")
                done <<< "$cands"
            fi
            # If every candidate ends with '/', it's a subdirectory — keep going
            # without inserting a trailing space so the user can continue typing.
            if [[ ${#COMPREPLY[@]} -gt 0 ]]; then
                local all_dirs=1
                local c
                for c in "${COMPREPLY[@]}"; do
                    [[ "$c" == */ ]] || { all_dirs=0; break; }
                done
                (( all_dirs )) && compopt -o nospace 2>/dev/null
            fi
        fi
        return 0
    fi

    # --- value completion for the previous flag ---
    case "$prev" in
        --output|-o)
            COMPREPLY=($(compgen -W "text json" -- "$cur"))
            return 0
            ;;
        --signing-version)
            COMPREPLY=($(compgen -W "v4 v2" -- "$cur"))
            return 0
            ;;
        --profile|--config|--region|--endpoint|--access-key|--secret-key|--path|--expires)
            # Free-form value; let bash fall back to filename completion.
            COMPREPLY=($(compgen -f -- "$cur"))
            return 0
            ;;
    esac

    # --- subcommand at position 1 ---
    # Walk the args to find the first non-flag, non-value token.
    local i sub=""
    for ((i=1; i<COMP_CWORD; i++)); do
        local w="${COMP_WORDS[i]}"
        case "$w" in
            -*) ;;
            obs://*|*/*|*.*) ;;
            *)
                # Check if previous was a value-taking flag
                local p="${COMP_WORDS[i-1]}"
                case "$p" in
                    --profile|--config|--region|--endpoint|--access-key|--secret-key|--output|-o|--signing-version|--path|--expires)
                        continue
                        ;;
                esac
                sub="$w"
                break
                ;;
        esac
    done

    if [[ -z "$sub" ]]; then
        if [[ "$cur" == -* ]]; then
            COMPREPLY=($(compgen -W "--help --version --profile --config --region --endpoint --access-key --secret-key --signing-version --insecure --output --quiet -o -q -h -V" -- "$cur"))
        else
            COMPREPLY=($(compgen -W "ls cp cat stat rm presign init completion help" -- "$cur"))
        fi
        return 0
    fi

    # --- per-subcommand flags ---
    if [[ "$cur" == -* ]]; then
        local global="--help --profile --config --region --endpoint --access-key --secret-key --signing-version --insecure --output --quiet -o -q -h"
        case "$sub" in
            ls)      COMPREPLY=($(compgen -W "$global --recursive -r" -- "$cur")) ;;
            rm)      COMPREPLY=($(compgen -W "$global --dry-run" -- "$cur")) ;;
            presign) COMPREPLY=($(compgen -W "$global --expires" -- "$cur")) ;;
            init)    COMPREPLY=($(compgen -W "--path --force --help -h" -- "$cur")) ;;
            completion) COMPREPLY=($(compgen -W "--help -h" -- "$cur")) ;;
            *)       COMPREPLY=($(compgen -W "$global" -- "$cur")) ;;
        esac
        return 0
    fi

    # --- positional values ---
    case "$sub" in
        completion)
            COMPREPLY=($(compgen -W "bash zsh" -- "$cur"))
            ;;
        cp)
            # cp takes <src> <dst>: local file OR obs:// URI. Offer both — obs://
            # gets the dynamic branch above when user types it; files get filename completion.
            COMPREPLY=($(compgen -f -- "$cur"))
            ;;
        *)
            # ls/cat/rm/stat/presign all take obs:// URIs.
            # Suggest "obs://" as a starting fragment when nothing typed.
            if [[ -z "$cur" ]]; then
                COMPREPLY=("obs://")
                compopt -o nospace 2>/dev/null
            fi
            ;;
    esac
}

complete -F _ya_obs ya-obs
