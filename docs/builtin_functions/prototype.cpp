#include "prototype.hpp"

void test_if() {
  std::variant<int, std::string> r1 = fn_if(
      /* condition */ true,
      /* then */ 1,
      /* else */ std::string{"hello"});
  assert(std::get<0>(r1) == 1);

  std::variant<int, std::string> r2 = fn_if(
      /* condition */ false,
      /* then */ 1,
      /* else */ std::string{"hello"});
  assert(std::get<1>(r2) == std::string("hello"));
}

void test_ifs() {
  std::variant<bool, std::string, int> r1 =
      fn_ifs<bool, std::pair<bool, std::string>, std::pair<bool, int>>(
          /* condition1, value1 */ std::pair{true, std::string{"123"}},
          /* condition2, value2 */ std::pair(true, 42),
          /* else */ false);
  assert(std::get<1>(r1) == std::string("123"));

  std::variant<bool, std::string, int> r2 =
      fn_ifs<bool, std::pair<bool, std::string>, std::pair<bool, int>>(
          /* condition1, value1 */ std::pair{false, std::string{"123"}},
          /* condition2, value2 */ std::pair(true, 42),
          /* else */ false);
  assert(std::get<2>(r2) == 42);
}

void test_sum() {
  using p = std::variant<int, List<int>>;

  int r1 = fn_sum(p{1});
  assert(r1 == 1);
  int r2 = fn_sum(p{List<int>{1, 2, 3}});
  assert(r2 == 6);
  int r3 = fn_sum(p{1}, p{List<int>{2, 3}}, p{4});
  assert(r3 == 10);
}

void test_length() {
  size_t r1 = fn_length(std::string{"hello"});
  assert(r1 == 5);

  size_t r2 = fn_length(std::vector<Any>{1, 2, std::string{"3"}});
  assert(r2 == 3);
}

void test_let() {
  int result = fn_let<int, int>(
      /* var, value */ "x"_ident, 1,
      /* expr */ [](int x) { return x + 2; });
  assert(result == 3);
}

void test_lets() {
  std::variant<int, std::string> r1 = fn_lets<
      std::function<std::variant<int, std::string>(int, std::string, bool)>,
      std::pair<std::string, int>, std::pair<std::string, std::string>,
      std::pair<std::string, bool>>(
      /* var1, value1 */ std::pair{"x"_ident, 1},
      /* var2, value2 */ std::pair{"y"_ident, "2"},
      /* var1, value1 */ std::pair{"z"_ident, true},
      /* expr */ [](int x, std::string y, bool z) {
        if (z) {
          return std::variant<int, std::string>{x};
        } else {
          return std::variant<int, std::string>{y};
        }
      });
  assert(std::get<0>(r1) == 1);
  std::variant<int, std::string> r2 = fn_lets<
      std::function<std::variant<int, std::string>(int, std::string, bool)>,
      std::pair<std::string, int>, std::pair<std::string, std::string>,
      std::pair<std::string, bool>>(
      /* var1, value1 */ std::pair{"x"_ident, 1},
      /* var2, value2 */ std::pair{"y"_ident, "2"},
      /* var3, value3 */ std::pair{"z"_ident, false},
      /* expr */ [](int x, std::string y, bool z) {
        if (z) {
          return std::variant<int, std::string>{x};
        } else {
          return std::variant<int, std::string>{y};
        }
      });
  assert(std::get<1>(r2) == std::string("2"));
}

/// Run the tests
int main() {
  test_if();
  test_ifs();
  test_sum();
  test_length();
  test_let();
  test_lets();

  return 0;
}
