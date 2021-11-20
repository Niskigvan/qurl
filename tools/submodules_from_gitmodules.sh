#!/bin/sh

#set -e

git config -f .gitmodules --get-regexp '^submodule\..*\.path$' |
    while read path_key path
    do
        url_key=$(echo $path_key | sed 's/\.path/.url/')
        url=$(git config -f .gitmodules --get "$url_key")
        branch_key=$(echo $path_key | sed 's/\.path/.branch/')
        branch=$(git config -f .gitmodules --get "$branch_key")
        if [ ! -d "$path" ]; then
            echo URL - $url, Path - $path, Branch - $branch
            if [ -n "$branch" ]; then
                branch="-b $branch"
            fi
            git submodule add --force $branch $url $path
        fi
    done