#compdef ya-obs
# zsh completion for ya-obs
# Install: ya-obs completion zsh > ~/.zfunc/_ya-obs
# Ensure ~/.zfunc is in your fpath, then run `compinit`.

_ya-obs() {
    local cur="${words[CURRENT]}"

    # --- dynamic obs:// path completion ---
    if [[ "$cur" == obs://* ]]; then
        local -a cands
        cands=("${(@f)$(command ya-obs --quiet __complete-obs -- "$cur" 2>/dev/null)}")
        if (( ${#cands[@]} )); then
            # If all end with /, suppress trailing space.
            local all_dirs=1 c
            for c in "${cands[@]}"; do
                [[ "$c" == */ ]] || { all_dirs=0; break }
            done
            if (( all_dirs )); then
                compadd -S '' -- "${cands[@]}"
            else
                compadd -- "${cands[@]}"
            fi
        fi
        return 0
    fi

    # --- find subcommand (first non-flag, non-value token after `ya-obs`) ---
    local sub="" i w p
    for (( i=2; i<CURRENT; i++ )); do
        w="${words[i]}"
        case "$w" in
            -*) ;;
            obs://*) ;;
            *)
                p="${words[i-1]}"
                case "$p" in
                    --profile|--config|--region|--endpoint|--access-key|--secret-key|--output|-o|--signing-version|--path|--expires) continue ;;
                esac
                sub="$w"
                break
                ;;
        esac
    done

    local prev="${words[CURRENT-1]}"
    case "$prev" in
        --output|-o)            _values 'output' text json; return 0 ;;
        --signing-version)      _values 'signing version' v4 v2; return 0 ;;
        --profile|--config|--region|--endpoint|--access-key|--secret-key|--path|--expires)
            _files
            return 0
            ;;
    esac

    if [[ -z "$sub" ]]; then
        if [[ "$cur" == -* ]]; then
            _values 'flag' \
                --help --version --profile --config --region --endpoint \
                --access-key --secret-key --signing-version --insecure \
                --output --quiet -o -q -h -V
        else
            _values 'subcommand' \
                'ls[List buckets or objects]' \
                'cp[Copy between local and obs://]' \
                'cat[Print object body]' \
                'stat[Fetch object metadata]' \
                'rm[Delete an object]' \
                'presign[Generate a presigned GET URL]' \
                'init[Write a starter config file]' \
                'completion[Print shell completion script]' \
                'help[Show help for a subcommand]'
        fi
        return 0
    fi

    # --- per-subcommand ---
    case "$sub" in
        completion)
            _values 'shell' bash zsh
            ;;
        cp)
            _files
            ;;
        ls|cat|rm|stat|presign)
            # Hint that an obs:// URI is expected when cur is empty.
            if [[ -z "$cur" ]]; then
                compadd -S '' -- 'obs://'
            fi
            ;;
    esac
}

_ya-obs "$@"
