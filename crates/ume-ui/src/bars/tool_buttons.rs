// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/uiparts/CMenuDrawer.cpp (the table `tbd` of `CMenuDrawer::CreateButtons`)
//   sakura_core/env/CShareData.cpp (`CShareData::InitToolButtons`: `DEFAULT_TOOL_BUTTONS`)
//
// Copyright (C) 1998-2001, Norio Nakatani
// Copyright (C) 2000, genta, Jepro
// Copyright (C) 2001, jepro, MIK, Misaka, YAZAKI, hor, genta
// Copyright (C) 2002, MIK, genta, YAZAKI, ai, Moca, hor, aroka
// Copyright (C) 2003, MIK, genta, Moca
// Copyright (C) 2004, Kazika, genta, Moca, isearch
// Copyright (C) 2005, genta, MIK, aroka
// Copyright (C) 2006, aroka, fon
// Copyright (C) 2007, ryoji
// Copyright (C) 2008, nasukoji
// Copyright (C) 2018-2026, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the names of the functions in the
// table were replaced by their function codes (`ume_commands::sakura::FUNCTIONS`), and the
// comments were rewritten. The table was extracted from the C++ source by a script; the
// function code of an entry is the one of its `F_` name in `Funccode_x.hsrc`.
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

//! The function of each toolbar button, and the buttons of the default toolbar.
//!
//! A toolbar button is a number: 0 is a separator, and the number N (1 or more) is the Nth entry
//! of Sakura's table, whose picture is the Nth icon (counted from 0) of `mytool.bmp`
//! (`CMenuDrawer::CreateButtons`: `m_tbMyButton[0]` is the separator, so a button number is the
//! icon number plus one). The settings file keeps these numbers (`[Toolbar]` `nTBB[000]`...).

/// Button number of the virtual button that wraps the toolbar to a new line
/// (`CMenuDrawer::TOOLBAR_BUTTON_F_TOOLBARWRAP`).
pub(super) const WRAP: u16 = 384;

/// How many buttons a toolbar can have (`MAX_TOOLBAR_BUTTON_ITEMS`).
pub(super) const MAX_BUTTONS: usize = 512;

/// The function code of the buttons 1..=481 (`tbd`). 0 is `F_DISABLE`: a button with no function
/// and no picture (a place kept free). The entry of [`WRAP`] is `F_TOOLBARWRAP`.
#[rustfmt::skip]
pub(super) const FUNCTIONS_OF_BUTTONS: [u16; 481] = [
    30101, 30102, 30103, 30104, 30105, 30107, 30111, 30112,
    30113, 30114, 30115, 30116, 30150, 30151, 30152, 30162,
    0, 0, 30170, 30171, 30180, 30190, 30185, 30117,
    30108, 30119, 30195, 30109, 30110, 30120, 30194, 30118,
    30210, 30211, 30221, 30222, 30230, 30231, 30233, 30232,
    30242, 30243, 30240, 30241, 30245, 30244, 30250, 30260,
    30261, 30262, 30263, 0, 30280, 30281, 30282, 30283,
    30284, 30285, 0, 0, 0, 0, 30191, 30122,
    30196, 30197, 31456, 30311, 30312, 30313, 30314, 30315,
    30316, 30320, 30321, 30332, 30333, 30340, 30341, 30342,
    30343, 30198, 31457, 30350, 30351, 30360, 30370, 30371,
    30380, 30381, 30382, 30383, 30372, 30394, 30393, 0,
    30400, 30401, 30410, 30411, 30412, 30413, 30414, 30415,
    30416, 30420, 30421, 30432, 30433, 30440, 30441, 30442,
    30443, 0, 0, 30450, 30451, 30482, 30483, 30403,
    30989, 30988, 0, 0, 0, 30485, 30484, 0,
    0, 0, 30510, 30511, 30512, 30513, 30514, 30515,
    30516, 30520, 30521, 30532, 30533, 30540, 30541, 30542,
    30543, 0, 0, 30550, 30551, 30530, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    30601, 30602, 30603, 30604, 30605, 0, 0, 30610,
    30611, 30612, 30620, 30621, 30630, 30622, 30608, 30613,
    30614, 30631, 0, 0, 0, 31081, 31082, 31083,
    30790, 30791, 30792, 30794, 30795, 0, 0, 0,
    30800, 30801, 30810, 30811, 30812, 30813, 30814, 30830,
    30850, 30851, 30852, 30853, 30854, 30855, 30860, 30861,
    30862, 30863, 30870, 30880, 30831, 30815, 30816, 30856,
    30817, 30990, 31104, 0, 0, 0, 30945, 30931,
    30901, 30902, 30903, 30904, 30905, 30910, 30920, 30930,
    30940, 30941, 30950, 30960, 30970, 30971, 30972, 30973,
    30974, 30976, 30978, 30979, 30980, 30908, 30909, 30943,
    30944, 30981, 30982, 30983, 30984, 30985, 30986, 30946,
    31001, 31099, 31010, 30912, 31100, 31101, 31102, 31110,
    31111, 31112, 31120, 31140, 31113, 31103, 0, 31431,
    31141, 31142, 31143, 31144, 31250, 31251, 31252, 31253,
    31270, 31254, 3, 0, 0, 0, 0, 31388,
    31580, 31501, 31502, 31503, 31504, 31505, 31506, 31507,
    31508, 31509, 31510, 31511, 31512, 31513, 31514, 31515,
    31516, 31517, 31518, 31519, 31520, 31521, 31522, 31523,
    31524, 31383, 31384, 31385, 31386, 31387, 31390, 31389,
    31310, 31311, 31312, 31320, 31321, 31340, 31341, 31330,
    31331, 31332, 31350, 31352, 31351, 31360, 31370, 31333,
    31334, 31337, 31336, 31380, 31381, 31382, 31430, 31440,
    31441, 31445, 31450, 31451, 31455, 0, 0, 0,
    31600, 31601, 31602, 31603, 31604, 31605, 31606, 31607,
    31608, 31609, 31610, 31611, 31612, 31613, 31614, 31615,
    31616, 31617, 31618, 31619, 31620, 31621, 31622, 31623,
    31624, 31625, 31626, 31627, 31628, 31629, 31630, 2,
    31631, 31632, 31633, 31634, 31635, 31636, 31637, 31638,
    31639, 31640, 31641, 31642, 31643, 31644, 31645, 31646,
    31647, 31648, 31649, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0,
];

/// The buttons of a toolbar that the settings file does not describe
/// (`CShareData::InitToolButtons`: `DEFAULT_TOOL_BUTTONS`).
pub(super) const DEFAULT_BUTTONS: [u16; 25] = [
    1, 25, 3, 4, 0, 33, 34, 0, 87, 88, 0, 225, 226, 227, 228, 229, 230, 0, 232, 0, 264, 265, 266,
    0, 346,
];
