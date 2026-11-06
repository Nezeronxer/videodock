cmake_minimum_required(VERSION 3.21)
project(videodock_appearance LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
find_package(Qt6 REQUIRED COMPONENTS Quick QuickControls2 Test WebEngineQuick Widgets)
qt_add_executable(videodock-appearance appearance.cpp)
target_link_libraries(videodock-appearance PRIVATE
    Qt6::Quick Qt6::QuickControls2 Qt6::Test Qt6::WebEngineQuick Qt6::Widgets)
