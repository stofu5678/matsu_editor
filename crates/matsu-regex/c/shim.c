/*
 * Thin C layer between Rust and Onigmo (see src/ffi.rs).
 *
 * It keeps the Rust side free of Onigmo's struct layouts and variadic
 * functions, and builds the regular expression syntax that Sakura Editor
 * users get from bregonig.dll.
 */
#include <stddef.h>
#include <string.h>

#include "onigmo.h"

/* Syntax kinds, mirrored in src/ffi.rs. */
enum {
  MATSU_SYNTAX_BREGONIG = 0,
  MATSU_SYNTAX_ONIGMO_DEFAULT = 1, /* Ruby syntax without ASCII_RANGE, as testpy.py */
  MATSU_SYNTAX_PERL = 2,
  MATSU_SYNTAX_GREP = 3
};

static OnigSyntaxType syntax_bregonig;
static OnigSyntaxType syntax_default;

int matsu_onig_init(void)
{
  int r = onig_init();
  if (r != ONIG_NORMAL) return r;

  /*
   * The syntax bregonig.html documents: ONIG_SYNTAX_PERL (section 5.1) plus
   * the three exceptions that section names, and the absent operator:
   * - ONIG_SYN_OP2_CCLASS_SET_OP: "&&" inside a character class (5.1, 5.1.6)
   * - ONIG_SYN_OP2_ESC_G_SUBEXP_CALL: "\g<name>" and "\g<n>" calls (5.1, 5.1.9)
   * - ONIG_SYN_DIFFERENT_LEN_ALT_LOOK_BEHIND: look-behind whose alternatives
   *   differ in length (5.1)
   * - ONIG_SYN_OP2_QMARK_TILDE_ABSENT: "(?~...)", since Ver.4.10 (5.1.7)
   * Everything else the document lists (\K, \R, \X, conditions, "(?P<n>)",
   * "(?&n)", "\g{n}") is already in ONIG_SYNTAX_PERL's bits (Onigmo regsyntax.c).
   */
  onig_copy_syntax(&syntax_bregonig, ONIG_SYNTAX_PERL);
  onig_set_syntax_op2(&syntax_bregonig,
      onig_get_syntax_op2(ONIG_SYNTAX_PERL)
      | ONIG_SYN_OP2_QMARK_TILDE_ABSENT
      | ONIG_SYN_OP2_ESC_G_SUBEXP_CALL
      | ONIG_SYN_OP2_CCLASS_SET_OP);
  onig_set_syntax_behavior(&syntax_bregonig,
      onig_get_syntax_behavior(ONIG_SYNTAX_PERL)
      | ONIG_SYN_DIFFERENT_LEN_ALT_LOOK_BEHIND);

  /* The syntax testpy.py uses for most cases. */
  onig_copy_syntax(&syntax_default, ONIG_SYNTAX_DEFAULT);
  onig_set_syntax_options(&syntax_default,
      onig_get_syntax_options(&syntax_default) & ~ONIG_OPTION_ASCII_RANGE);
  return ONIG_NORMAL;
}

static const OnigSyntaxType* syntax_of(int kind)
{
  switch (kind) {
    case MATSU_SYNTAX_PERL: return ONIG_SYNTAX_PERL;
    case MATSU_SYNTAX_GREP: return ONIG_SYNTAX_GREP;
    case MATSU_SYNTAX_ONIGMO_DEFAULT: return &syntax_default;
    default: return &syntax_bregonig;
  }
}

/*
 * Option bits for the letters of bregonig.html section 5.2 that Sakura uses:
 * i, x, m, a, R. Sakura builds the option string in CBregexp::MakePatternSub
 * (sakura_core/extmodule/CBregexp.cpp): always k and m, plus i, g, x, a, u, d,
 * l and R as the user's search options say.
 */
unsigned int matsu_opt_ignorecase(void) { return ONIG_OPTION_IGNORECASE; }
unsigned int matsu_opt_extend(void) { return ONIG_OPTION_EXTEND; }
unsigned int matsu_opt_negate_singleline(void) { return ONIG_OPTION_NEGATE_SINGLELINE; }
unsigned int matsu_opt_ascii_range(void) { return ONIG_OPTION_ASCII_RANGE; }
unsigned int matsu_opt_newline_crlf(void) { return ONIG_OPTION_NEWLINE_CRLF; }

/*
 * Compiles a UTF-8 pattern. On failure returns the Onigmo error code (< 0)
 * and writes the message to msg (ONIG_MAX_ERROR_MESSAGE_LEN bytes at least).
 */
int matsu_regex_new(OnigRegex* out, const unsigned char* pattern, size_t len,
                  unsigned int options, int syntax_kind, unsigned char* msg)
{
  OnigErrorInfo einfo;
  int r = onig_new(out, pattern, pattern + len, options, ONIG_ENCODING_UTF8,
                   syntax_of(syntax_kind), &einfo);
  if (r != ONIG_NORMAL) {
    *out = NULL;
    onig_error_code_to_str(msg, r, &einfo);
  }
  return r;
}

void matsu_regex_free(OnigRegex reg) { onig_free(reg); }

int matsu_regex_captures(OnigRegex reg) { return onig_number_of_captures(reg); }

OnigRegion* matsu_region_new(void) { return onig_region_new(); }

void matsu_region_free(OnigRegion* region) { onig_region_free(region, 1); }

/* Returns the number of regions (groups + 1) after a successful search. */
int matsu_region_len(const OnigRegion* region) { return region->num_regs; }

/* Returns 0 and the byte range of group i, or -1 if the group did not match. */
int matsu_region_get(const OnigRegion* region, int i, ptrdiff_t* beg, ptrdiff_t* end)
{
  if (i < 0 || i >= region->num_regs || region->beg[i] == ONIG_REGION_NOTPOS) return -1;
  *beg = region->beg[i];
  *end = region->end[i];
  return 0;
}

/*
 * Searches str[0..len) from start towards range (range < start searches
 * backward). gpos < 0 means "no \G position" (Onigmo then uses start).
 * Returns the match start, ONIG_MISMATCH (-1), or an error code (< -1);
 * the message of an error goes to msg.
 */
ptrdiff_t matsu_regex_search(OnigRegex reg, const unsigned char* str, size_t len,
                           ptrdiff_t gpos, size_t start, size_t range,
                           unsigned int options, OnigRegion* region,
                           unsigned char* msg)
{
  ptrdiff_t r;
  if (gpos >= 0) {
    r = onig_search_gpos(reg, str, str + len, str + gpos, str + start,
                         str + range, region, options);
  } else {
    r = onig_search(reg, str, str + len, str + start, str + range, region,
                    options);
  }
  if (r < ONIG_MISMATCH) onig_error_code_to_str(msg, r);
  return r;
}

/* Anchored match at `at`. Returns the match length, -1, or an error code. */
ptrdiff_t matsu_regex_match(OnigRegex reg, const unsigned char* str, size_t len,
                          size_t at, unsigned int options, OnigRegion* region,
                          unsigned char* msg)
{
  ptrdiff_t r = onig_match(reg, str, str + len, str + at, region, options);
  if (r < ONIG_MISMATCH) onig_error_code_to_str(msg, r);
  return r;
}

int matsu_set_match_stack_limit(unsigned int size) { return onig_set_match_stack_limit_size(size); }
int matsu_set_parse_depth_limit(unsigned int depth) { return onig_set_parse_depth_limit(depth); }
