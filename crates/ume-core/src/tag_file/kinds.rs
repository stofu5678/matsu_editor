// Ported in part from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/dlg/CDlgTagJumpList.cpp (the table of kinds, GetNameByType)
//
// sakura_core/dlg/CDlgTagJumpList.cpp:
//   Copyright (C) 2003, MIK
//   Copyright (C) 2005, MIK
//   Copyright (C) 2006, genta, ryoji
//   Copyright (C) 2010, Moca
//   Copyright (C) 2018-2026, Sakura Editor Organization
//
// This file is derived from the file above and was modified: the table of file extensions and
// kinds of tags was moved into Rust data, the lookup was rewritten, and the tests are new.
//
// zlib License
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! The "種類" column of the tag jump list: the letter that `ctags` writes for the kind of a tag,
//! as a word, by the extension of the file (`GetNameByType`).

const C_KINDS: &str = "c=class,d=macro,e=enumerator,f=function,g=enum,m=member,n=namespace,p=prototype,s=struct,t=typedef,u=union,v=variable,x=externvar";

/// The extensions of a language and the kinds that `ctags` knows for it, in Sakura's order.
const LANGUAGES: &[(&str, &str)] = &[
    ("asm,s", "d=define,l=label,m=macro,t=type"),
    ("asp,asa", "f=function,s=sub"),
    ("awk,gawk,mawk", "f=function"),
    ("bet", "f=fragment,p=pattern,s=slot,v=virtual"),
    ("c,h", C_KINDS),
    ("c++,cc,cp,cpp,cxx,h++,hh,hp,hpp,hxx", C_KINDS),
    ("java", C_KINDS),
    ("vr,vri,vrh", C_KINDS),
    (
        "cbl,cob",
        "d=data,f=file,g=group,p=paragraph,P=program,s=section",
    ),
    ("e", "c=class,f=feature,l=local"),
    (
        "f,for,ftn,f77,f90,f95",
        "b=block data,c=common,e=entry,f=function,i=interface,k=component,l=label,L=local,m=module,n=namelist,p=program,s=subroutine,t=type,v=variable",
    ),
    ("cl,clisp,el,l,lisp,lsp,ml", "f=function"),
    ("lua", "f=function"),
    ("mak", "m=macro"),
    ("p,pas", "f=function,p=procedure"),
    ("pl,pm,perl", "s=subroutine,p=package"),
    ("php,php3,phtml", "c=class,f=function"),
    ("py,python", "c=class,f=function,m=member"),
    ("cmd,rexx,rx", "s=subroutine"),
    ("rb", "c=class,f=method,F=singleton method,m=mixin"),
    ("sch,scheme,scm,sm", "f=function,s=set"),
    ("sh,bsh,bash,ksh,zsh", "f=function"),
    ("sl", "f=function,n=namespace"),
    (
        "sql",
        "c=cursor,d=prototype,f=function,F=field,l=local,P=package,p=procedure,r=record,s=subtype,t=table,T=trigger,v=variable",
    ),
    ("tcl,tk,wish,itcl", "p=procedure,c=class,f=method"),
    (
        "v",
        "f=function,m=module,P=parameter,p=port,r=reg,t=task,v=variable,w=wire",
    ),
    ("vim", "f=function,v=variable"),
    ("y", "l=label"),
];

/// The name of the kind `kind` of a tag in `file`, or an empty string when `ctags` has no such
/// kind for the language of the file. A file without an extension is taken as `.c`.
pub fn kind_name(kind: Option<char>, file: &str) -> &'static str {
    let ext = file.rsplit_once('.').map_or("c", |(_, ext)| ext);
    let Some((_, kinds)) = LANGUAGES
        .iter()
        .find(|(exts, _)| exts.split(',').any(|e| e.eq_ignore_ascii_case(ext)))
    else {
        return "";
    };
    let Some(kind) = kind else {
        return "";
    };
    kinds
        .split(',')
        .find_map(|entry| {
            let (letter, name) = entry.split_once('=')?;
            letter.starts_with(kind).then_some(name)
        })
        .unwrap_or("")
}
