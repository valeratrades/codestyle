# Everything `cl review` draws from. An entry is either a `skill` (naming <dir>/SKILL.md next to this
# file) or a standalone `prompt` handed to the agent as-is.
#
# `importance` is 1.0..=3.0, default 1.0. Raising it spaces that entry's own runs more evenly, and
# wins it a larger share of runs against lower-set entries. See pick.rs for the sampling itself.
[
  {
    importance = 2.0;
    skill = "codebase_primitives";
  }

  {
    importance = 1.5;
    prompt = ''
      Find the single worst public function in this codebase that should be private or removed
      entirely, and refactor to shrink the module boundary. Remove > refactor > add.
    '';
  }
  {
    prompt = ''
      Find a self-contained piece of complex functionality in a large file, and nest it in an inlined
      module. This reduces entropy, cause piceces of complex machinery there suddenly don't mingle.
    '';
  }
  {
    importance = 1.5;
    prompt = ''
      Hunt for one fallback that masks tainted state (unwrap_or, let _ =, silent default) and replace
      it with a loud panic/error at the earliest point the state goes bad.
    '';
  }
  {
    prompt = ''
      Find a place where an invariant is assumed but not asserted, and add the assert. Pick the
      assertion that would catch the nastiest latent bug.
    '';
  }
  {
    importance = 1.5;
    prompt = ''
      Locate the most duplicated logic in the codebase and collapse it into a single source of truth,
      preferring std trait impls (From, etc) over a new helper fn.
    '';
  }
  {
    prompt = ''
      Find the worst error-handling site (a swallowed error, a vague message, a stringly-typed error)
      and improve it with thiserror/miette/proper context.
    '';
  }
  {
    prompt = ''
      Pick the most confusingly-named symbol in the codebase and rename it to something that matches
      what it actually does. Update all call sites.
    '';
  }
  {
    importance = 1.5;
    prompt = ''
      Find dead code, unused pub items, or unreachable branches and delete them. Verify nothing
      depends on them first.
    '';
  }
  {
    prompt = ''
      Pick a module and go through tests there. For each you justify why it should be kept. The
      default action is deletion. Goal is to eliminate all that are tautalogical (eg some logic
      upstream sets a code to a color, and then in the test we go through the codes and check the
      colors. This literally adds no value, and must be gone, - things like that)
    '';
  }
]
