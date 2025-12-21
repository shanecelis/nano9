-- copyright (c) 2023 shane celis[1]
-- licensed under the mit license[2]
--
-- this is the tinytest library
-- for pico-8; it was inspired
-- by the tinytest[3] javascript
-- library by joe walnes. it
-- provides a basic unit test
-- framework.
--
-- you can use it one of two
-- ways: as a no frills library
-- or as a singing, dancing cart.
--
-- library usage
-- -------------
--
-- you will enjoy colored text
-- reports but otherwise no
-- frills, but it's very
-- flexible this way.
--
-- ```
-- #include tinytest.p8:0
--
-- tinytest:new():run({
--   demo_pass = function(t)
--     t:ok(true, "hi")
--   end,
--
--   demo_fail = function(t)
--     t:ok(false, "bye")
--   end,
--
--   demo_error = function(t)
--     assert(false, "wtf")
--   end,
--
--   demo_misc  =
--   function(t)
--     t:ok(false, "bye2")
--     assert(false, "wtf2")
--   end,
-- })
-- ```
--
-- cart usage
-- ----------
--
-- the cart comes with some
-- images of bob from the
-- incredibles in meme format
-- and some audio sfx so you can
-- hear the sweet sound of tests
-- passing, failing, and
-- erroring to make unit testing
-- more fun.
--
-- in your cart, define
-- `my_tinytests`:
--
-- ```
-- -- yourcart.p8
-- my_tinytests = {
--   demo_pass = function(t)
--     t:ok(true, "yep")
--   end
-- }
-- ```
--
-- edit tinytest.p8's cart:
--
-- ```
-- -- tinytest.p8
-- #include yourcart.p8
-- ```
--
-- load tinytest.p8 and on every
-- run it will exercise your
-- tests. since it does an
-- include, you don't have to
-- reload either.
--
-- todo
-- ====
--
-- * add more bob meme images
--   (only two currently)
-- *
--
-- [1]: https://mastodon.gamedev.place/@shanecelis
-- [2]: https://opensource.org/licenses/MIT
-- [3]: https://github.com/joewalnes/jstinytest


-- define my_tintests in yourcart.
-- #include yourcart.p8
-- #include lib/matrix.p8
-- try runs the given function
-- t() first. on errors call
-- c(e). finally call f() when
-- complete.
--
-- try from https://github.com/sparr/pico8lib/blob/master/functions.p8
--
-- there is also a trace
-- function for coroutines gives
-- a stacktrace in the preceding
-- link.
local function try(t, c, f)
  local co = cocreate(t)
  local s, e = true
  while s and costatus(co) ~= "dead" do
    s, e = coresume(co)
    if not s then
      c(e)
    end
  end
  if f then
    f()
  end
end

-- tinytest class
--
-- it can be extended. see
-- bobtest below.
tinytest = {

  new = function(self, o)
    o = o or {}
    setmetatable(o, self)
    self.__index = self
    -- o.verbose = o.verbose or false
    o.failures = {}
    o.fail_is_error = o.fail_is_error or false
    return o
  end,

  -- utility function. counts
  -- the number of entries in a
  -- table. sequences can do
  -- #list but tables have to be
  -- iterated it seems.
  table_count = function(table)
    local count = 0
    for key, value in pairs(table) do
      count = count + 1
    end
    return count
  end,

  -- run the tests and return
  -- the tables failures and
  -- errors.
  run = function(self, tests)
    local errors_map = {}
    local failures_map = {}
    local errors = {}
    local fails = 0
    local errs = 0
    cls()
    print("test results: \0")
    for testname, testaction in pairs(tests) do
      try(function ()
            testaction(self)
          end,
          function (e)
            add(errors, e)
          end,
          function ()

          end)

      if #errors == 0 and #self.failures == 0 then
        print("p\0")
      end

      if #self.failures ~= 0 then
        print("f\0")
        failures_map[testname] = self.failures
        self.failures = {}

        fails = fails + 1
      end
      if #errors ~= 0 then
        print("e\0")
        errors_map[testname] = errors
        errors = {}
        errs = errs + 1
      end
    end
    print(nil,0)
    for testname, testaction in pairs(tests) do
      print(testname)
      if failures_map[testname] or errors_map[testname] then
        for _, failure in ipairs(failures_map[testname]) do
          print("  " .. failure)
          world.error(testname .. ": " .. failure)
        end

        for _, error in ipairs(errors_map[testname]) do
          -- print("  error line " .. sub(error,32))
          print("  error line " .. error)
          world.error(testname .. ": " .. error)
          print(error)
        end
      else
        print("  pass")
      end
    end
    return fails, errs, failures_map, errors_map
  end,

  -- report an unconditional failure.
  fail = function(self, msg, header)
    if not msg then
      msg = ''
    end
    header = header or 'fail'
    if self.false_is_error then
      assert(false,      header .. msg)
    else
      add(self.failures, header .. msg)
    end
  end,

  -- assert something is true.
  ok = function(self, value, msg)
      if not value then self:fail(msg, '\fanot ok: \f6') end
  end,

  -- assert something is equal.
  eq = function(self, expected, actual, msg)
      if expected ~= actual then self:fail('"' .. expected .. '" ~= "' .. actual .. '" '..(msg or ''), 'not eq: ') end
  end,

}
