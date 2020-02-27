if has("autocmd")
	augroup tmpl
		autocmd BufNewFile *.rs 0r .vim/template.rs
	augroup END
endif
