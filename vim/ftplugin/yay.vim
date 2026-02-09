" Vim filetype plugin
" Language:    YAY (Yet Another YAML)
" Maintainer:  Kris Kowal
" License:     Apache 2.0

if exists('b:did_ftplugin')
  finish
endif
let b:did_ftplugin = 1

" YAY uses two-space indentation.
setlocal expandtab
setlocal shiftwidth=2
setlocal softtabstop=2
setlocal tabstop=2

" Comment format for 'commentary' and similar plugins.
setlocal commentstring=#\ %s
setlocal comments=:#

" Undo settings when switching filetype.
let b:undo_ftplugin = 'setlocal expandtab< shiftwidth< softtabstop< tabstop<'
      \ . ' commentstring< comments< foldmethod<'
